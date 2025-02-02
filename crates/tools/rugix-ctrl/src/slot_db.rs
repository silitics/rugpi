//! Slot database.

use std::path::{Path, PathBuf};

use reportify::ResultExt;
use rugix_bundle::block_encoding::block_index::{compute_block_index, BlockIndexConfig};
use rugix_bundle::manifest::ChunkerAlgorithm;
use rugix_common::system::SystemResult;
use rugix_hashes::HashAlgorithm;
use tracing::warn;

/// Stored block index.
#[derive(Debug)]
pub struct StoredBlockIndex {
    /// Chunker algorithm.
    pub chunker_algorithm: ChunkerAlgorithm,
    /// Hash algorithm.
    pub hash_algorithm: HashAlgorithm,
    /// Path to the file containing the index.
    pub index_file: PathBuf,
}

pub fn add_index(
    slot_name: &str,
    slot_file: &Path,
    chunker_algorithm: &ChunkerAlgorithm,
    hash_algorithm: &HashAlgorithm,
) -> SystemResult<()> {
    let path = db_dir().join(format!(
        "{slot_name}/{chunker_algorithm}_{}.rugix-block-index",
        hash_algorithm.name(),
    ));
    std::fs::create_dir_all(path.parent().unwrap()).ok();
    let index_config = BlockIndexConfig {
        hash_algorithm: *hash_algorithm,
        chunker: chunker_algorithm.clone(),
    };
    let block_index =
        compute_block_index(index_config, slot_file).whatever("unable to compute block index")?;
    std::fs::write(path, &block_index.encode()).whatever("unable to write block index")?;
    Ok(())
}

/// Get the stored block indices.
pub fn get_stored_indices(slot: &str) -> SystemResult<Vec<StoredBlockIndex>> {
    let slot_dir = db_dir().join(slot);
    let mut indices = Vec::new();
    if slot_dir.exists() {
        for dir_entry in std::fs::read_dir(&slot_dir).whatever("unable to list index directory")? {
            let dir_entry = dir_entry.whatever("unable to list indices directory")?;
            let filename = dir_entry.file_name();
            let filename = filename.to_string_lossy();
            let Some(name) = filename.strip_suffix(".rugix-block-index") else {
                continue;
            };
            let Some((chunker_algorithm, hash_algorithm)) = name.split_once('_') else {
                warn!("invalid filename for block index: {filename:?}");
                continue;
            };
            let Ok(chunker_algorithm) = chunker_algorithm.parse() else {
                warn!("invalid chunker algorithm: {chunker_algorithm:?}");
                continue;
            };
            let Ok(hash_algorithm) = hash_algorithm.parse() else {
                warn!("invalid hash algorithm: {hash_algorithm:?}");
                continue;
            };
            indices.push(StoredBlockIndex {
                chunker_algorithm,
                hash_algorithm,
                index_file: dir_entry.path(),
            })
        }
    }
    Ok(indices)
}

/// Directory with the slot database.
pub fn db_dir() -> &'static Path {
    const DATA_PATH: &str = "/run/rugix/mounts/data/rugix/slots";
    const VAR_PATH: &str = "/var/rugix/slots";
    if Path::new("/run/rugix/mounts/data").exists() {
        Path::new(DATA_PATH)
    } else {
        Path::new(VAR_PATH)
    }
}
