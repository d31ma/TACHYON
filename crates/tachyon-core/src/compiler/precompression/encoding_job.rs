use super::{CompressionCache, ContentEncoding, PreviousOutput};
use std::io;
use std::path::{Path, PathBuf};

pub(super) struct EncodingJob<'a> {
    pub(super) file: &'a Path,
    pub(super) bytes: &'a [u8],
    pub(super) encoding: ContentEncoding,
}

impl EncodingJob<'_> {
    pub(super) fn run(&self, previous: &PreviousOutput) -> io::Result<Option<(PathBuf, Vec<u8>)>> {
        let encoded = previous
            .reusable_variant(self.file, self.bytes, self.encoding)
            .map_or_else(
                || CompressionCache::global().encode(self.bytes, self.encoding),
                Ok,
            )?;
        let smaller = encoded.len() < self.bytes.len();
        Ok(smaller.then(|| (self.encoding.variant_path(self.file), encoded)))
    }
}
