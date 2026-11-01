use sha2::{Digest, Sha256};
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

pub(crate) struct RustWasmCache {
    unversioned: PathBuf,
    folder: PathBuf,
    entry: PathBuf,
}

impl RustWasmCache {
    const ENTRY_FORMAT_VERSION: u32 = 2;
    const FOLDER: &str = "rust-wasm";
    const DIGEST_BYTES: usize = 32;
    const PRE_VERSIONED_FRAME: &[u8] = b"tachyon-rust-wasm-1\0";

    pub(crate) fn folder(cache_root: &Path) -> PathBuf {
        cache_root
            .join(Self::FOLDER)
            .join(format!("v{}", Self::ENTRY_FORMAT_VERSION))
    }

    pub(super) fn new(cache_root: &Path, key: &str) -> Self {
        let folder = Self::folder(cache_root);
        Self {
            unversioned: cache_root.join(Self::FOLDER),
            entry: folder.join(format!("{key}.wasm")),
            folder,
        }
    }

    fn format() -> Vec<u8> {
        format!("tachyon-rust-wasm-{}\0", Self::ENTRY_FORMAT_VERSION).into_bytes()
    }

    pub(super) fn read(&self) -> Option<Vec<u8>> {
        let stored = std::fs::read(&self.entry).ok()?;
        let rest = stored.strip_prefix(Self::format().as_slice())?;
        let (digest, module) = rest.split_at_checked(Self::DIGEST_BYTES)?;
        (Sha256::digest(module).as_slice() == digest).then(|| module.to_vec())
    }

    pub(super) fn store(&self, module: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.folder)?;
        let mut staged = tempfile::Builder::new()
            .prefix(".saving-")
            .tempfile_in(&self.folder)?;
        staged.write_all(&Self::format())?;
        staged.write_all(&Sha256::digest(module))?;
        staged.write_all(module)?;
        staged.as_file().sync_all()?;
        staged
            .persist(&self.entry)
            .map(drop)
            .map_err(|failure| failure.error)?;
        self.remove_unversioned_frames();
        Ok(())
    }

    fn remove_unversioned_frames(&self) {
        let entries = std::fs::read_dir(&self.unversioned).into_iter().flatten();
        for entry in entries.flatten() {
            Self::remove_unversioned_frame(&entry.path());
        }
    }

    fn remove_unversioned_frame(path: &Path) {
        if Self::starts_with_unversioned_frame(path) {
            let _ = std::fs::remove_file(path);
        }
    }

    fn starts_with_unversioned_frame(path: &Path) -> bool {
        let mut start = [0; Self::PRE_VERSIONED_FRAME.len()];
        std::fs::File::open(path)
            .and_then(|mut file| file.read_exact(&mut start))
            .is_ok_and(|()| start == Self::PRE_VERSIONED_FRAME)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::RustWasmCache;
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    impl RustWasmCache {
        const MODULE: &[u8] = b"\0asm\x01\0\0\0services";
    }

    fn stored_cache(root: &std::path::Path) -> RustWasmCache {
        let cache = RustWasmCache::new(root, "key");
        cache.store(RustWasmCache::MODULE).expect("store");
        cache
    }

    fn unversioned_entry(root: &std::path::Path, name: &str) -> std::path::PathBuf {
        let folder = root.join(RustWasmCache::FOLDER);
        std::fs::create_dir_all(&folder).expect("unversioned folder");
        folder.join(name)
    }

    fn names(folder: &std::path::Path) -> Vec<String> {
        std::fs::read_dir(folder)
            .expect("list")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn a_stored_module_reads_back_and_leaves_no_staging_file() {
        let root = tempfile::tempdir().expect("root");
        let cache = stored_cache(root.path());
        assert_eq!(cache.read().as_deref(), Some(RustWasmCache::MODULE));
        assert_eq!(names(&cache.folder), ["key.wasm"]);
    }

    #[test]
    fn entries_land_only_in_the_versioned_folder() {
        let root = tempfile::tempdir().expect("root");
        let cache = stored_cache(root.path());
        assert_eq!(
            cache.entry,
            root.path()
                .join("rust-wasm")
                .join(format!("v{}", RustWasmCache::ENTRY_FORMAT_VERSION))
                .join("key.wasm")
        );
        assert_eq!(
            names(&root.path().join("rust-wasm")),
            [format!("v{}", RustWasmCache::ENTRY_FORMAT_VERSION)]
        );
    }

    #[test]
    fn a_raw_module_from_an_older_ty_is_ignored_and_kept_for_it() {
        let root = tempfile::tempdir().expect("root");
        let older = unversioned_entry(root.path(), "key.wasm");
        std::fs::write(&older, b"\0asm\x01\0\0\0older").expect("older entry");
        let cache = RustWasmCache::new(root.path(), "key");
        assert_eq!(cache.read(), None);
        cache.store(RustWasmCache::MODULE).expect("rebuild");
        assert_eq!(cache.read().as_deref(), Some(RustWasmCache::MODULE));
        assert_eq!(
            std::fs::read(&older).expect("older entry"),
            b"\0asm\x01\0\0\0older"
        );
    }

    #[test]
    fn a_store_removes_unversioned_frames_an_older_ty_would_serve() {
        let root = tempfile::tempdir().expect("root");
        let framed = unversioned_entry(root.path(), "other.wasm");
        let mut frame = RustWasmCache::PRE_VERSIONED_FRAME.to_vec();
        frame.extend_from_slice(&Sha256::digest(RustWasmCache::MODULE));
        frame.extend_from_slice(RustWasmCache::MODULE);
        std::fs::write(&framed, frame).expect("framed entry");
        let short = unversioned_entry(root.path(), "short.wasm");
        std::fs::write(&short, b"\0asm").expect("short entry");
        stored_cache(root.path());
        assert!(!framed.exists());
        assert!(short.exists());
    }

    #[test]
    fn the_entry_layout_is_pinned_to_its_format_version() {
        let root = tempfile::tempdir().expect("root");
        let cache = stored_cache(root.path());
        let stored = std::fs::read(&cache.entry).expect("entry");
        let digest = Sha256::digest(stored)
            .iter()
            .fold(String::new(), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            });
        assert_eq!(
            (RustWasmCache::ENTRY_FORMAT_VERSION, digest.as_str()),
            (
                2,
                "09867a43dea942cb670d816e2a465d2e9182f44ead3b4bab864e4841143e4831"
            ),
            "the entry layout changed: bump RustWasmCache::ENTRY_FORMAT_VERSION so older and \
             newer ty builds never read each other's entries, then update this pin"
        );
    }

    #[test]
    fn a_torn_entry_is_not_used_and_the_rebuild_replaces_it() {
        let root = tempfile::tempdir().expect("root");
        let cache = stored_cache(root.path());
        let stored = std::fs::read(&cache.entry).expect("entry");
        for length in [0, 1, RustWasmCache::format().len() + 4, stored.len() - 1] {
            std::fs::write(&cache.entry, &stored[..length]).expect("tear");
            assert_eq!(cache.read(), None, "a {length}-byte entry was used");
        }
        cache.store(RustWasmCache::MODULE).expect("rebuild");
        assert_eq!(cache.read().as_deref(), Some(RustWasmCache::MODULE));
    }

    #[test]
    fn an_entry_whose_module_changed_is_not_used() {
        let root = tempfile::tempdir().expect("root");
        let cache = stored_cache(root.path());
        let mut stored = std::fs::read(&cache.entry).expect("entry");
        let last = stored.len() - 1;
        stored[last] ^= 1;
        std::fs::write(&cache.entry, stored).expect("corrupt");
        assert_eq!(cache.read(), None);
    }

    #[test]
    fn an_unframed_module_in_the_versioned_folder_is_not_used() {
        let root = tempfile::tempdir().expect("root");
        let cache = RustWasmCache::new(root.path(), "key");
        std::fs::create_dir_all(&cache.folder).expect("folder");
        std::fs::write(&cache.entry, RustWasmCache::MODULE).expect("unframed");
        assert_eq!(cache.read(), None);
    }

    #[test]
    fn concurrent_stores_always_leave_a_whole_entry() {
        let root = tempfile::tempdir().expect("root");
        let module = RustWasmCache::MODULE.repeat(4096);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    let cache = RustWasmCache::new(root.path(), "key");
                    for _ in 0..8 {
                        let _ = cache.store(&module);
                        assert!(cache.read().is_none_or(|read| read == module));
                    }
                });
            }
        });
        let cache = RustWasmCache::new(root.path(), "key");
        assert_eq!(cache.read(), Some(module));
    }
}
