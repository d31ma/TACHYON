use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tachyon_compression::BrotliEncoder;

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

    pub(super) fn matches(self, encoded: &[u8], source: &[u8]) -> bool {
        self.reader(encoded)
            .is_some_and(|reader| Self::decoded_matches(reader, source))
    }

    fn reader(self, encoded: &[u8]) -> Option<Box<dyn Read + '_>> {
        match self.extension {
            "br" => Some(Box::new(brotli::Decompressor::new(encoded, 4096))),
            "gz" => Some(Box::new(flate2::read::GzDecoder::new(encoded))),
            _ => None,
        }
    }

    fn decoded_matches(reader: Box<dyn Read + '_>, source: &[u8]) -> bool {
        let mut decoded = Vec::new();
        let read = reader
            .take(source.len() as u64 + 1)
            .read_to_end(&mut decoded);
        read.is_ok() && decoded == source
    }

    fn brotli(bytes: &[u8]) -> io::Result<Vec<u8>> {
        BrotliEncoder::new(Self::BROTLI_QUALITY, Self::BROTLI_WINDOW_BITS).compress(bytes)
    }

    fn gzip(bytes: &[u8]) -> io::Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(bytes)?;
        encoder.finish()
    }
}
