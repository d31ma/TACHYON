use sha2::{Digest, Sha256};
use std::io::Write as _;
use std::path::{Path, PathBuf};

pub(super) struct RustWasmCache {
    folder: PathBuf,
    entry: PathBuf,
}

impl RustWasmCache {
    const FORMAT: &[u8] = b"tachyon-rust-wasm-1\0";
    const DIGEST_BYTES: usize = 32;

    pub(super) fn new(folder: &Path, key: &str) -> Self {
        Self {
            folder: folder.to_path_buf(),
            entry: folder.join(format!("{key}.wasm")),
        }
    }

    pub(super) fn read(&self) -> Option<Vec<u8>> {
        let stored = std::fs::read(&self.entry).ok()?;
        let rest = stored.strip_prefix(Self::FORMAT)?;
        let (digest, module) = rest.split_at_checked(Self::DIGEST_BYTES)?;
        (Sha256::digest(module).as_slice() == digest).then(|| module.to_vec())
    }

    pub(super) fn store(&self, module: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.folder)?;
        let mut staged = tempfile::Builder::new()
            .prefix(".saving-")
            .tempfile_in(&self.folder)?;
        staged.write_all(Self::FORMAT)?;
        staged.write_all(&Sha256::digest(module))?;
        staged.write_all(module)?;
        staged.as_file().sync_all()?;
        staged
            .persist(&self.entry)
            .map(drop)
            .map_err(|failure| failure.error)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::RustWasmCache;

    impl RustWasmCache {
        const MODULE: &[u8] = b"\0asm\x01\0\0\0services";
    }

    fn stored_cache(folder: &std::path::Path) -> RustWasmCache {
        let cache = RustWasmCache::new(folder, "key");
        cache.store(RustWasmCache::MODULE).expect("store");
        cache
    }

    #[test]
    fn a_stored_module_reads_back_and_leaves_no_staging_file() {
        let folder = tempfile::tempdir().expect("folder");
        let cache = stored_cache(folder.path());
        assert_eq!(cache.read().as_deref(), Some(RustWasmCache::MODULE));
        let names: Vec<_> = std::fs::read_dir(folder.path())
            .expect("list")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(names, ["key.wasm"]);
    }

    #[test]
    fn a_torn_entry_is_not_used_and_the_rebuild_replaces_it() {
        let folder = tempfile::tempdir().expect("folder");
        let cache = stored_cache(folder.path());
        let stored = std::fs::read(&cache.entry).expect("entry");
        for length in [0, 1, RustWasmCache::FORMAT.len() + 4, stored.len() - 1] {
            std::fs::write(&cache.entry, &stored[..length]).expect("tear");
            assert_eq!(cache.read(), None, "a {length}-byte entry was used");
        }
        cache.store(RustWasmCache::MODULE).expect("rebuild");
        assert_eq!(cache.read().as_deref(), Some(RustWasmCache::MODULE));
    }

    #[test]
    fn an_entry_whose_module_changed_is_not_used() {
        let folder = tempfile::tempdir().expect("folder");
        let cache = stored_cache(folder.path());
        let mut stored = std::fs::read(&cache.entry).expect("entry");
        let last = stored.len() - 1;
        stored[last] ^= 1;
        std::fs::write(&cache.entry, stored).expect("corrupt");
        assert_eq!(cache.read(), None);
    }

    #[test]
    fn an_unframed_module_from_an_older_release_is_not_used() {
        let folder = tempfile::tempdir().expect("folder");
        let cache = RustWasmCache::new(folder.path(), "key");
        std::fs::write(&cache.entry, RustWasmCache::MODULE).expect("legacy");
        assert_eq!(cache.read(), None);
    }

    #[test]
    fn concurrent_stores_always_leave_a_whole_entry() {
        let folder = tempfile::tempdir().expect("folder");
        let module = RustWasmCache::MODULE.repeat(4096);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    let cache = RustWasmCache::new(folder.path(), "key");
                    for _ in 0..8 {
                        let _ = cache.store(&module);
                        assert!(cache.read().is_none_or(|read| read == module));
                    }
                });
            }
        });
        let cache = RustWasmCache::new(folder.path(), "key");
        assert_eq!(cache.read(), Some(module));
    }
}
