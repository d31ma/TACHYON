use super::CacheDirectory;
use std::ffi::OsStr;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(super) struct SharedHandlerArtifacts {
    folder: PathBuf,
}

impl SharedHandlerArtifacts {
    const FOLDER: &'static str = "runtime/handlers-v1";
    const MAX_ENTRIES: usize = 512;
    const PENDING_PREFIX: &'static str = ".pending-";

    pub(super) fn in_user_cache() -> Self {
        Self::at(crate::cache::root().join(Self::FOLDER))
    }

    pub(super) const fn at(folder: PathBuf) -> Self {
        Self { folder }
    }

    pub(super) fn restore(&self, cache: &CacheDirectory, artefact: &Path) -> bool {
        let shared = self.shared_regular_file(artefact);
        shared.is_some_and(|path| cache.publish_file(&path, artefact).is_ok())
    }

    pub(super) fn publish(&self, cache: &CacheDirectory, artefact: &Path) {
        let _ = self.copy_out(cache, artefact);
        self.keep_newest();
    }

    fn shared_regular_file(&self, artefact: &Path) -> Option<PathBuf> {
        let path = self.folder.join(artefact.file_name()?);
        let metadata = fs::symlink_metadata(&path).ok()?;
        (metadata.is_file() && metadata.len() > 0).then_some(path)
    }

    fn copy_out(&self, cache: &CacheDirectory, artefact: &Path) -> std::io::Result<()> {
        let name = artefact
            .file_name()
            .ok_or_else(|| std::io::Error::other("the handler artifact has no name"))?;
        let pending = self.pending_path(name);
        fs::create_dir_all(&self.folder)?;
        let copied = cache
            .copy_file_out(artefact, &pending)
            .and_then(|()| fs::rename(&pending, self.folder.join(name)));
        copied.inspect_err(|_| Self::forget(&pending))
    }

    fn pending_path(&self, name: &OsStr) -> PathBuf {
        let pending = format!(
            "{}{}.{}",
            Self::PENDING_PREFIX,
            name.to_string_lossy(),
            std::process::id()
        );
        self.folder.join(pending)
    }

    fn keep_newest(&self) {
        let mut entries = self.finished_entries();
        entries.sort();
        let extra = entries.len().saturating_sub(Self::MAX_ENTRIES);
        for (_, path) in entries.into_iter().take(extra) {
            Self::forget(&path);
        }
    }

    fn finished_entries(&self) -> Vec<(SystemTime, PathBuf)> {
        fs::read_dir(&self.folder)
            .map(|listing| listing.flatten().filter_map(Self::finished_entry).collect())
            .unwrap_or_default()
    }

    fn finished_entry(entry: DirEntry) -> Option<(SystemTime, PathBuf)> {
        let metadata = entry.metadata().ok()?;
        let modified = metadata.modified().ok()?;
        let name = entry.file_name();
        let pending = name.to_string_lossy().starts_with(Self::PENDING_PREFIX);
        (metadata.is_file() && !pending).then_some((modified, entry.path()))
    }

    fn forget(path: &Path) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used)]
mod tests {
    use super::SharedHandlerArtifacts;
    use crate::handler::cache::CacheDirectory;
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::Path;

    fn compiled_file(folder: &Path, bytes: &[u8]) -> std::path::PathBuf {
        let path = folder.join("output.bin");
        fs::write(&path, bytes).expect("compiler output");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("mode");
        path
    }

    #[test]
    fn a_handler_built_in_one_project_is_restored_in_another() {
        let shared_folder = tempfile::tempdir().expect("shared");
        let shared = SharedHandlerArtifacts::at(shared_folder.path().join("handlers"));
        let first = tempfile::tempdir().expect("first project");
        let second = tempfile::tempdir().expect("second project");
        let artefact = Path::new("abc123.bin");
        let first_cache = CacheDirectory::open(first.path()).expect("first cache");
        let output = compiled_file(first.path(), b"compiled handler");
        first_cache.publish_file(&output, artefact).expect("built");
        shared.publish(&first_cache, artefact);

        let second_cache = CacheDirectory::open(second.path()).expect("second cache");
        assert!(shared.restore(&second_cache, artefact));
        let restored = second.path().join(".tachyon/handlers/abc123.bin");
        assert_eq!(fs::read(&restored).expect("restored"), b"compiled handler");
        let mode = fs::metadata(&restored).expect("mode").permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "the restored handler stays executable");
    }

    #[test]
    fn a_missing_empty_or_linked_shared_entry_is_not_restored() {
        let shared_folder = tempfile::tempdir().expect("shared");
        let shared = SharedHandlerArtifacts::at(shared_folder.path().to_path_buf());
        let project = tempfile::tempdir().expect("project");
        let cache = CacheDirectory::open(project.path()).expect("cache");
        assert!(!shared.restore(&cache, Path::new("missing.bin")));

        fs::write(shared_folder.path().join("empty.bin"), b"").expect("empty");
        assert!(!shared.restore(&cache, Path::new("empty.bin")));

        let outside = compiled_file(project.path(), b"outside");
        std::os::unix::fs::symlink(&outside, shared_folder.path().join("linked.bin"))
            .expect("link");
        assert!(!shared.restore(&cache, Path::new("linked.bin")));
        assert!(!project.path().join(".tachyon/handlers/linked.bin").exists());
    }

    #[test]
    fn the_shared_folder_keeps_only_the_newest_entries() {
        let shared_folder = tempfile::tempdir().expect("shared");
        let shared = SharedHandlerArtifacts::at(shared_folder.path().to_path_buf());
        for index in 0..SharedHandlerArtifacts::MAX_ENTRIES {
            fs::write(
                shared_folder.path().join(format!("old-{index}.bin")),
                b"old",
            )
            .expect("old entry");
        }
        let project = tempfile::tempdir().expect("project");
        let cache = CacheDirectory::open(project.path()).expect("cache");
        let output = compiled_file(project.path(), b"new");
        let artefact = Path::new("new.bin");
        cache.publish_file(&output, artefact).expect("built");
        std::thread::sleep(std::time::Duration::from_millis(20));
        shared.publish(&cache, artefact);

        let count = fs::read_dir(shared_folder.path()).expect("listing").count();
        assert_eq!(count, SharedHandlerArtifacts::MAX_ENTRIES);
        assert!(shared_folder.path().join("new.bin").is_file());
    }
}
