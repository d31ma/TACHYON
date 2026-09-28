use super::content_digest::ContentDigest;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct TypeScriptCache;

impl TypeScriptCache {
    const EMIT_SETTINGS: &'static str = "es2022 esnext bundler noCheck ignoreConfig";

    pub(super) fn entry(program: &Path, source: &Path, source_bytes: &[u8]) -> Option<PathBuf> {
        let installed = fs::canonicalize(program).ok()?;
        let metadata = fs::metadata(&installed).ok()?;
        let modified = metadata
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        let key = Self::hash_parts(&[
            Self::EMIT_SETTINGS.as_bytes(),
            installed.to_string_lossy().as_bytes(),
            metadata.len().to_string().as_bytes(),
            modified.as_nanos().to_string().as_bytes(),
            source.file_name()?.to_string_lossy().as_bytes(),
            source_bytes,
        ]);
        Some(
            crate::cache::root()
                .join("typescript")
                .join(format!("{key}.js")),
        )
    }

    pub(super) fn read(entry: Option<&Path>) -> Option<Vec<u8>> {
        entry.and_then(|entry| fs::read(entry).ok())
    }

    pub(super) fn store(entry: Option<&Path>, bytes: &[u8]) {
        let folder_ready = entry
            .and_then(Path::parent)
            .is_some_and(|folder| fs::create_dir_all(folder).is_ok());
        Self::store_when_ready(entry.filter(|_| folder_ready), bytes);
    }

    fn hash_parts(parts: &[&[u8]]) -> String {
        let mut hasher = Sha256::new();
        for part in parts {
            hasher.update(part);
            hasher.update([0]);
        }
        ContentDigest::hex(hasher.finalize())
    }

    fn store_when_ready(entry: Option<&Path>, bytes: &[u8]) {
        if let Some(entry) = entry {
            Self::write_entry(entry, bytes);
        }
    }

    fn write_entry(entry: &Path, bytes: &[u8]) {
        let partial = entry.with_extension(format!("{}.partial", std::process::id()));
        let written = fs::write(&partial, bytes).is_ok();
        Self::publish_partial(written, &partial, entry);
    }

    fn publish_partial(written: bool, partial: &Path, entry: &Path) {
        if written && fs::rename(partial, entry).is_err() {
            let _ = fs::remove_file(partial);
        }
    }
}
