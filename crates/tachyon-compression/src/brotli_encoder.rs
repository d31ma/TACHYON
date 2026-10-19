use brotli::enc::BrotliEncoderParams;
use std::io;

pub struct BrotliEncoder {
    quality: i32,
    window_bits: i32,
}

impl BrotliEncoder {
    #[must_use]
    pub const fn new(quality: i32, window_bits: i32) -> Self {
        Self {
            quality,
            window_bits,
        }
    }

    pub fn compress(&self, bytes: &[u8]) -> io::Result<Vec<u8>> {
        let params = BrotliEncoderParams {
            quality: self.quality,
            lgwin: self.window_bits,
            size_hint: bytes.len(),
            ..BrotliEncoderParams::default()
        };
        let mut encoded = Vec::new();
        brotli::BrotliCompress(&mut &bytes[..], &mut encoded, &params)?;
        Ok(encoded)
    }
}
