use super::ContentEncoding;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(super) struct CompressionCache {
    root: PathBuf,
}

impl CompressionCache {
    const MAX_CACHE_BYTES: u64 = 256 * 1024 * 1024;
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
        Self::read_regular(&path, valid, &key)
    }

    fn read_regular(path: &Path, valid: bool, key: &[u8; 32]) -> Option<Vec<u8>> {
        valid.then_some(())?;
        let mut bytes = Vec::new();
        File::open(path)
            .ok()?
            .take(Self::MAX_ENTRY_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        let entry = Self::matching_entry(bytes, key)?;
        Self::mark_used(path);
        Some(entry)
    }

    fn mark_used(path: &Path) {
        let touched = File::options()
            .write(true)
            .open(path)
            .and_then(|file| file.set_modified(SystemTime::now()));
        drop(touched);
    }

    fn matching_entry(bytes: Vec<u8>, key: &[u8; 32]) -> Option<Vec<u8>> {
        let fits = bytes.len() <= Self::MAX_ENTRY_BYTES;
        let matches = bytes.get(..Self::KEY_BYTES) == Some(key.as_slice());
        (fits && matches).then_some(bytes)
    }

    fn fresh(&self, bytes: &[u8], encoding: ContentEncoding) -> io::Result<Vec<u8>> {
        let encoded = encoding.encode(bytes)?;
        let written = self.persist(bytes, &encoded, encoding);
        drop(written);
        Ok(encoded)
    }

    pub(super) fn trim(&self) {
        self.trim_to(Self::MAX_CACHE_BYTES);
    }

    pub(super) fn trim_to(&self, budget: u64) {
        let entries = self.entries_oldest_first();
        let total: u64 = entries.iter().map(|(_, size, _)| size).sum();
        Self::remove_oldest(entries, total.saturating_sub(budget));
    }

    fn entries_oldest_first(&self) -> Vec<(SystemTime, u64, PathBuf)> {
        let mut entries: Vec<(SystemTime, u64, PathBuf)> = fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| Self::entry(&entry))
            .collect();
        entries.sort();
        entries
    }

    fn entry(entry: &fs::DirEntry) -> Option<(SystemTime, u64, PathBuf)> {
        let metadata = entry.metadata().ok().filter(fs::Metadata::is_file)?;
        Some((metadata.modified().ok()?, metadata.len(), entry.path()))
    }

    fn remove_oldest(entries: Vec<(SystemTime, u64, PathBuf)>, excess: u64) {
        let mut counted = 0_u64;
        entries
            .into_iter()
            .take_while(|(_, size, _)| {
                let more = counted < excess;
                counted += size;
                more
            })
            .for_each(|(_, _, path)| drop(fs::remove_file(path)));
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
                .variant_path(Path::new("content"))
                .as_os_str()
                .as_encoded_bytes(),
        );
        hash.update(bytes);
        hash.finalize().into()
    }

    fn path(&self, key: &[u8; 32], encoding: ContentEncoding) -> PathBuf {
        let name = key.iter().fold(String::new(), |mut name, byte| {
            let _ = write!(name, "{byte:02x}");
            name
        });
        self.root.join(encoding.variant_path(Path::new(&name)))
    }
}
