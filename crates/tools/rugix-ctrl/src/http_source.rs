use std::io::Read;

use byte_calc::NumBytes;
use reportify::{bail, ResultExt};
use rugix_bundle::source::BundleSource;
use rugix_common::system::SystemResult;
use ureq::http::Response;
use ureq::Body;

pub struct HttpSource {
    url: String,
    supports_range: bool,
    current_response: Response<Body>,
    current_position: u64,
    current_skipped: u64,
    skip_buffer: Vec<u8>,
}

impl HttpSource {
    pub fn new(url: &str) -> SystemResult<Self> {
        let response = ureq::get(url)
            .call()
            .whatever("unable to get bundle from URL")?;
        Ok(Self {
            url: url.to_owned(),
            supports_range: response
                .headers()
                .get("Accept-Ranges")
                .map(|value| value.as_bytes() == b"bytes")
                .unwrap_or(false),
            current_response: response,
            current_skipped: 0,
            current_position: 0,
            skip_buffer: Vec::new(),
        })
    }
}

impl BundleSource for HttpSource {
    fn read(&mut self, slice: &mut [u8]) -> rugix_bundle::BundleResult<usize> {
        if self.current_skipped > 0 {
            self.current_position += self.current_skipped;
            if self.current_skipped > NumBytes::kibibytes(32) && self.supports_range {
                self.current_response = ureq::get(&self.url)
                    .header("Range", format!("bytes={}-", self.current_position))
                    .call()
                    .whatever("unable to get bundle from URL")?;
            } else {
                let mut remaining = self.current_skipped;
                while remaining > 0 {
                    self.skip_buffer.resize(remaining.min(8192) as usize, 0);
                    let read = self
                        .current_response
                        .body_mut()
                        .as_reader()
                        .read(&mut self.skip_buffer)
                        .whatever("unable to read from HTTP source")?;
                    if read == 0 {
                        bail!("unexpected end of HTTP stream")
                    }
                    remaining -= read as u64;
                }
            }
            self.current_skipped = 0;
        }
        let read = self
            .current_response
            .body_mut()
            .as_reader()
            .read(slice)
            .whatever("unable to read from HTTP source")?;
        self.current_position += read as u64;
        Ok(read)
    }

    fn skip(&mut self, length: byte_calc::NumBytes) -> rugix_bundle::BundleResult<()> {
        self.current_skipped += length.raw;
        Ok(())
    }
}
