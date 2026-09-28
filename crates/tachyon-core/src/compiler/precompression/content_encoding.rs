use brotli::enc::BrotliEncoderParams;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub(super) struct ContentEncoding {
    extension: &'static str,
    encoder: fn(&[u8]) -> io::Result<Vec<u8>>,
}

impl ContentEncoding {
    pub(super) const BROTLI: Self = Self {
        extension: "br",
        encoder: Self::brotli,
    };
    pub(super) const GZIP: Self = Self {
        extension: "gz",
        encoder: Self::gzip,
    };
    pub(super) const ALL: [Self; 2] = [Self::BROTLI, Self::GZIP];
    const BROTLI_QUALITY: i32 = 11;
    const BROTLI_WINDOW_BITS: i32 = 22;

    pub(super) fn variant_path(self, file: &Path) -> PathBuf {
        let mut name = file.as_os_str().to_owned();
        name.push(".");
        name.push(self.extension);
        PathBuf::from(name)
    }

    pub(super) fn encode(self, bytes: &[u8]) -> io::Result<Vec<u8>> {
        (self.encoder)(bytes)
    }

    fn brotli(bytes: &[u8]) -> io::Result<Vec<u8>> {
        let params = BrotliEncoderParams {
            quality: Self::BROTLI_QUALITY,
            lgwin: Self::BROTLI_WINDOW_BITS,
            size_hint: bytes.len(),
            ..BrotliEncoderParams::default()
        };
        let mut encoded = Vec::new();
        brotli::BrotliCompress(&mut &bytes[..], &mut encoded, &params)?;
        Ok(encoded)
    }

    fn gzip(bytes: &[u8]) -> io::Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(bytes)?;
        encoder.finish()
    }
}
