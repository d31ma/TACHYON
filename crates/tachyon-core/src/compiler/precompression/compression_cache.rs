use super::ContentEncoding;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;

pub(super) struct CompressionCache {
    root: PathBuf,
}

impl CompressionCache {
    const SLOTS_PER_ENCODING: u8 = 32;
    const KEY_BYTES: usize = 32;
    const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
    const MAX_ENCODED_BYTES: usize = 4 * 1024 * 1024;
    const MAX_ENTRY_BYTES: usize = Self::MAX_ENCODED_BYTES + Self::KEY_BYTES;
    const VERSION: &'static [u8] = b"brotli9-q11-w22:flate2-1.1.10-best:v1";

    pub(super) fn global() -> Self {
        Self::at(crate::cache::root().join("runtime/precompression-v1"))
    }

    pub(super) fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub(super) fn encode(&self, bytes: &[u8], encoding: ContentEncoding) -> io::Result<Vec<u8>> {
        self.cached(bytes, encoding)
            .map_or_else(|| self.fresh(bytes, encoding), Ok)
    }

    fn cached(&self, bytes: &[u8], encoding: ContentEncoding) -> Option<Vec<u8>> {
        let entry = self.read_entry(bytes, encoding)?;
        let encoded = entry.get(Self::KEY_BYTES..)?;
        let valid = encoding.matches(encoded, bytes);
        valid.then(|| encoded.to_vec())
    }

    fn read_entry(&self, bytes: &[u8], encoding: ContentEncoding) -> Option<Vec<u8>> {
        (bytes.len() <= Self::MAX_SOURCE_BYTES).then_some(())?;
        let key = Self::key(bytes, encoding);
        let path = self.path(&key, encoding);
        let metadata = fs::symlink_metadata(&path).ok()?;
        let valid = metadata.is_file() && metadata.len() <= Self::MAX_ENTRY_BYTES as u64;
        Self::read_regular(path, valid, &key)
    }

    fn read_regular(path: PathBuf, valid: bool, key: &[u8; 32]) -> Option<Vec<u8>> {
        valid.then_some(())?;
        let mut bytes = Vec::new();
        File::open(path)
            .ok()?
            .take(Self::MAX_ENTRY_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        Self::matching_entry(bytes, key)
    }

    fn matching_entry(bytes: Vec<u8>, key: &[u8; 32]) -> Option<Vec<u8>> {
        let fits = bytes.len() <= Self::MAX_ENTRY_BYTES;
        let matches = bytes.get(..Self::KEY_BYTES) == Some(key.as_slice());
        (fits && matches).then_some(bytes)
    }

    fn fresh(&self, bytes: &[u8], encoding: ContentEncoding) -> io::Result<Vec<u8>> {
        let encoded = encoding.encode(bytes)?;
        self.persist(bytes, &encoded, encoding)?;
        Ok(encoded)
    }

    fn persist(&self, bytes: &[u8], encoded: &[u8], encoding: ContentEncoding) -> io::Result<()> {
        if bytes.len() > Self::MAX_SOURCE_BYTES || encoded.len() > Self::MAX_ENCODED_BYTES {
            return Ok(());
        }
        self.write_entry(bytes, encoded, encoding)
    }

    fn write_entry(
        &self,
        bytes: &[u8],
        encoded: &[u8],
        encoding: ContentEncoding,
    ) -> io::Result<()> {
        let key = Self::key(bytes, encoding);
        fs::create_dir_all(&self.root)?;
        let mut pending = tempfile::NamedTempFile::new_in(&self.root)?;
        pending.write_all(&key)?;
        pending.write_all(encoded)?;
        pending
            .persist(self.path(&key, encoding))
            .map_err(|error| error.error)?;
        Ok(())
    }

    fn key(bytes: &[u8], encoding: ContentEncoding) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(Self::VERSION);
        hash.update(
            encoding
                .variant_path(std::path::Path::new("content"))
                .as_os_str()
                .as_encoded_bytes(),
        );
        hash.update(bytes);
        hash.finalize().into()
    }

    fn path(&self, key: &[u8; 32], encoding: ContentEncoding) -> PathBuf {
        let slot = key[0] % Self::SLOTS_PER_ENCODING;
        let name = format!("slot-{slot:02}");
        self.root
            .join(encoding.variant_path(std::path::Path::new(&name)))
    }
}
