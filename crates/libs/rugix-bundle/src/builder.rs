use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use byte_calc::NumBytes;
use reportify::ResultExt;
use rugix_hashes::HashDigest;

use crate::block_encoding::encode_payload_file;
use crate::format::stlv::{write_atom_head, write_segment_end, write_segment_start};
use crate::format::{self, Bytes, PayloadEntry, PayloadHeader};
use crate::manifest::{self, BundleManifest, HashAlgorithm, UpdateType};
use crate::BundleResult;

pub fn pack(path: &Path, dst: &Path) -> BundleResult<()> {
    let manifest = toml::from_str::<BundleManifest>(
        &std::fs::read_to_string(path.join("rugix-bundle.toml"))
            .whatever("unable to read bundle manifest")?,
    )
    .whatever("unable to parse bundle manifest")?;
    let hash_algorithm = manifest
        .hash_algorithm
        .unwrap_or(rugix_hashes::HashAlgorithm::Sha512_256);
    let mut bundle_header = format::BundleHeader {
        manifest: Some(serde_json::to_string(&manifest).unwrap()),
        is_incremental: matches!(manifest.update_type, UpdateType::Incremental),
        hash_algorithm,
        payload_index: Vec::new(),
    };
    let mut prepared_payloads = Vec::new();
    for (idx, payload) in manifest.payloads.iter().enumerate() {
        let payload_file = path.join("payloads").join(&payload.filename);
        let payload_file_hash =
            hash_file(hash_algorithm, &payload_file).whatever("unable to hash payload file")?;
        let mut payload_data = payload_file.clone();
        let mut payload_header = PayloadHeader {
            block_encoding: None,
        };
        if let Some(block_encoding) = &payload.block_encoding {
            payload_data = path.join(format!(".payload{idx}.data"));
            payload_header.block_encoding = Some(encode_payload_file(
                block_encoding,
                &payload_file,
                &payload_data,
            )?);
        }
        let payload_header = format::encode::to_vec(&payload_header, format::tags::PAYLOAD_HEADER);
        bundle_header.payload_index.push(PayloadEntry {
            type_slot: if let manifest::DeliveryConfig::Slot(slot_config) = &payload.delivery {
                Some(format::SlotPayloadType {
                    slot: slot_config.slot.clone(),
                })
            } else {
                None
            },
            type_execute: if let manifest::DeliveryConfig::Execute(execute_delivery_config) =
                &payload.delivery
            {
                Some(format::ExecutePayloadType {
                    handler: execute_delivery_config.handler.clone(),
                })
            } else {
                None
            },
            header_hash: Bytes {
                raw: hash_algorithm.hash(&payload_header).raw().to_vec(),
            },
            file_hash: Bytes {
                raw: payload_file_hash.raw().to_vec(),
            },
        });
        prepared_payloads.push(PreparedPayload {
            payload_header,
            payload_data,
        })
    }
    let mut bundle_file =
        BufWriter::new(std::fs::File::create(dst).whatever("unable to create bundle file")?);
    write_segment_start(&mut bundle_file, format::tags::BUNDLE).unwrap();
    let bundle_header = format::encode::to_vec(&bundle_header, format::tags::BUNDLE_HEADER);
    let header_hash = hash_algorithm.hash(&bundle_header);
    bundle_file.write_all(&bundle_header).unwrap();
    write_segment_start(&mut bundle_file, format::tags::PAYLOADS).unwrap();
    for prepared in prepared_payloads.into_iter() {
        write_segment_start(&mut bundle_file, format::tags::PAYLOAD).unwrap();
        bundle_file.write_all(&prepared.payload_header).unwrap();
        let data_size = std::fs::metadata(&prepared.payload_data).unwrap().len();
        write_atom_head(
            &mut bundle_file,
            format::stlv::AtomHead::Value {
                tag: format::tags::PAYLOAD_DATA,
                length: NumBytes::new(data_size),
            },
        )
        .unwrap();
        let mut payload_data = std::fs::File::open(&prepared.payload_data).unwrap();
        std::io::copy(&mut payload_data, &mut bundle_file).unwrap();
        write_segment_end(&mut bundle_file, format::tags::PAYLOAD).unwrap();
    }
    write_segment_end(&mut bundle_file, format::tags::PAYLOADS).unwrap();
    write_segment_end(&mut bundle_file, format::tags::BUNDLE).unwrap();
    println!("{header_hash}");
    Ok(())
}

struct PreparedPayload {
    payload_header: Vec<u8>,
    payload_data: PathBuf,
}

fn hash_file(algorithm: HashAlgorithm, path: &Path) -> std::io::Result<HashDigest> {
    let mut hasher = algorithm.hasher();
    let mut reader = BufReader::new(std::fs::File::open(path)?);
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            break Ok(hasher.finalize());
        }
        hasher.update(buffer);
        let consumed = buffer.len();
        reader.consume(consumed);
    }
}
