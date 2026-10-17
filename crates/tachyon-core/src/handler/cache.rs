use cap_fs_ext::{DirExt as _, FollowSymlinks, MetadataExt as _, OpenOptionsFollowExt as _};
#[cfg(test)]
use cap_std::ambient_authority;
use cap_std::fs::{Dir, Metadata, OpenOptions};
use sha2::{Digest as _, Sha256};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Component, Path, PathBuf};
#[cfg(unix)]
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Debug)]
pub(super) struct CacheDirectory {
    directory: Arc<Dir>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EntryIdentity {
    device: u64,
    inode: u64,
}

impl EntryIdentity {
    fn of(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

#[derive(Debug)]
pub(super) struct OwnedCacheEntry {
    cache: CacheDirectory,
    relative: PathBuf,
    identity: EntryIdentity,
    published: bool,
}

#[derive(Debug)]
pub(super) struct CacheLock {
    cache: CacheDirectory,
    relative: PathBuf,
    token: String,
    identity: EntryIdentity,
}

#[derive(Debug)]
struct CacheEntry {
    relative: PathBuf,
    name: String,
    metadata: Metadata,
}

#[derive(Debug)]
struct PrunableEntry {
    relative: PathBuf,
    name: String,
    modified: cap_std::time::SystemTime,
    usage: (usize, u64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LockSnapshot {
    identity: EntryIdentity,
    fingerprint: [u8; 32],
    owner: Option<LockOwner>,
    modified: Duration,
}

impl LockSnapshot {
    fn same_lock_as(&self, other: &Self) -> bool {
        (self.identity, self.fingerprint) == (other.identity, other.fingerprint)
    }

    fn read_metadata(file: &mut cap_std::fs::File) -> std::io::Result<(Vec<u8>, bool)> {
        let limit = CacheLock::MAX_METADATA_BYTES;
        let mut parser_bytes = Vec::with_capacity(limit + 1);
        std::io::Read::by_ref(file)
            .take((limit + 1) as u64)
            .read_to_end(&mut parser_bytes)?;
        let overflowed = parser_bytes.len() > limit;
        parser_bytes.truncate(limit);
        Ok((parser_bytes, overflowed))
    }

    fn fingerprint_of(
        identity: EntryIdentity,
        length: u64,
        modified: Duration,
        overflowed: bool,
        parser_bytes: &[u8],
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(identity.device.to_le_bytes());
        hasher.update(identity.inode.to_le_bytes());
        hasher.update(length.to_le_bytes());
        hasher.update(modified.as_secs().to_le_bytes());
        hasher.update(modified.subsec_nanos().to_le_bytes());
        hasher.update([u8::from(overflowed)]);
        hasher.update(parser_bytes);
        hasher.finalize().into()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LockOwner {
    pid: u32,
    created: u64,
    token: String,
}

impl LockOwner {
    fn parse(contents: &str) -> Option<Self> {
        let mut fields = contents.split_whitespace();
        let pid = fields.next()?;
        let created = fields.next()?;
        let token = fields.next()?;
        if fields.next().is_some() {
            return None;
        }
        let pid = pid.parse::<u32>().ok()?;
        let created = created.parse::<u64>().ok()?;
        Some(Self {
            pid,
            created,
            token: token.to_string(),
        })
    }
}

impl CacheDirectory {
    const MAX_ENTRIES: usize = 256;
    const MAX_BYTES: u64 = 512 * 1024 * 1024;
    const LOCK_RETRY: Duration = Duration::from_millis(10);
    const EXISTING_STAGE_ERRORS: [&'static str; 2] = [
        "cache child is a symlink or unsupported type",
        "cached stage does not match its content identity",
    ];
    const COMPETING_STAGE_ERRORS: [&'static str; 2] = [
        "competing cache stage is not the expected file",
        "competing cache stage is not the expected file",
    ];

    #[cfg(all(test, unix))]
    pub(super) fn open(project_root: &Path) -> std::io::Result<Self> {
        let project = Dir::open_ambient_dir(project_root, ambient_authority())?;
        Self::open_project(&project)
    }

    pub(super) fn open_project(project: &Dir) -> std::io::Result<Self> {
        Self::create_directory_if_missing(project, Path::new(".tachyon"))?;
        let tachyon = project.open_dir_nofollow(".tachyon")?;
        Self::create_directory_if_missing(&tachyon, Path::new("handlers"))?;
        let handlers = tachyon.open_dir_nofollow("handlers")?;
        Ok(Self {
            directory: Arc::new(handlers),
        })
    }

    #[cfg(test)]
    pub(super) fn open_test_root(path: &Path) -> std::io::Result<Self> {
        Dir::open_ambient_dir(path, ambient_authority()).map(|directory| Self {
            directory: Arc::new(directory),
        })
    }

    pub(super) fn metadata(&self, relative: &Path) -> std::io::Result<Metadata> {
        let (parent, name) = self.parent_and_name(relative)?;
        parent.symlink_metadata(name)
    }

    pub(super) fn read(&self, relative: &Path) -> std::io::Result<Vec<u8>> {
        let mut file = self.open_regular_file(relative)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    pub(super) fn read_to_string(&self, relative: &Path) -> std::io::Result<String> {
        String::from_utf8(self.read(relative)?).map_err(|error| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, error.utf8_error())
        })
    }

    pub(super) fn is_valid_file(&self, relative: &Path) -> bool {
        self.metadata(relative)
            .is_ok_and(|metadata| Self::is_filled_regular_file(&metadata))
    }

    fn is_filled_regular_file(metadata: &Metadata) -> bool {
        metadata.is_file() && !metadata.is_symlink() && metadata.len() > 0
    }

    pub(super) fn create_owned_directory(
        &self,
        relative: &Path,
    ) -> std::io::Result<OwnedCacheEntry> {
        let (parent, name) = self.parent_and_name(relative)?;
        parent.create_dir(&name)?;
        let identity = EntryIdentity::of(&parent.symlink_metadata(&name)?);
        Ok(OwnedCacheEntry {
            cache: self.clone(),
            relative: relative.to_path_buf(),
            identity,
            published: false,
        })
    }

    #[cfg(test)]
    pub(super) fn adopt_file(&self, relative: &Path) -> std::io::Result<OwnedCacheEntry> {
        let identity = self.identity(relative)?;
        Ok(OwnedCacheEntry {
            cache: self.clone(),
            relative: relative.to_path_buf(),
            identity,
            published: false,
        })
    }

    #[cfg(test)]
    pub(super) fn adopt_directory(&self, relative: &Path) -> std::io::Result<OwnedCacheEntry> {
        let identity = self.identity(relative)?;
        Ok(OwnedCacheEntry {
            cache: self.clone(),
            relative: relative.to_path_buf(),
            identity,
            published: false,
        })
    }

    #[cfg(test)]
    pub(super) fn write_owned_for_test(
        &self,
        relative: &Path,
        bytes: &[u8],
    ) -> std::io::Result<OwnedCacheEntry> {
        self.write_owned_file(relative, bytes)
    }

    pub(super) fn create_directory(&self, relative: &Path) -> std::io::Result<()> {
        let (parent, name) = self.parent_and_name(relative)?;
        parent.create_dir(&name)
    }

    pub(super) fn stage_bytes(&self, final_path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let existing = Self::tolerating(self.metadata(final_path), std::io::ErrorKind::NotFound)?;
        existing.map_or_else(
            || self.write_stage(final_path, bytes),
            |metadata| self.check_stage(&metadata, final_path, bytes, Self::EXISTING_STAGE_ERRORS),
        )
    }

    fn write_stage(&self, final_path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let temporary = Self::sibling_temporary(final_path, "stage")?;
        let guard = self.write_owned_file(&temporary, bytes)?;
        let linked = Self::tolerating(
            self.hard_link(&temporary, final_path),
            std::io::ErrorKind::AlreadyExists,
        )?;
        linked.map_or_else(|| self.check_competing_stage(final_path, bytes), Ok)?;
        drop(guard);
        Ok(())
    }

    fn check_competing_stage(&self, final_path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let metadata = self.metadata(final_path)?;
        self.check_stage(&metadata, final_path, bytes, Self::COMPETING_STAGE_ERRORS)
    }

    fn check_stage(
        &self,
        metadata: &Metadata,
        final_path: &Path,
        bytes: &[u8],
        [kind_error, content_error]: [&str; 2],
    ) -> std::io::Result<()> {
        Self::require(Self::is_regular_file(metadata), kind_error)?;
        let same_content = self.read(final_path)? == bytes;
        Self::require(same_content, content_error)
    }

    fn is_regular_file(metadata: &Metadata) -> bool {
        metadata.is_file() && !metadata.is_symlink()
    }

    fn require(condition: bool, message: &str) -> std::io::Result<()> {
        condition
            .then_some(())
            .ok_or_else(|| Self::invalid_data(message))
    }

    fn regular_component(component: Component<'_>) -> std::io::Result<OsString> {
        let Component::Normal(value) = component else {
            return Err(Self::invalid_data(
                "cache paths must be relative and regular",
            ));
        };
        Ok(value.to_os_string())
    }

    fn tolerating<T>(
        result: std::io::Result<T>,
        tolerated: std::io::ErrorKind,
    ) -> std::io::Result<Option<T>> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.kind() == tolerated => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub(super) fn publish_file(&self, source: &Path, final_path: &Path) -> std::io::Result<()> {
        let source_metadata = fs::symlink_metadata(source)?;
        Self::require(
            Self::is_filled_source_file(&source_metadata),
            "compiler output is not a non-empty regular file",
        )?;
        let temporary = Self::sibling_temporary(final_path, "publish")?;
        let guard = self.copy_owned_file(source, &temporary)?;
        self.hard_link(&temporary, final_path)?;
        drop(guard);
        Ok(())
    }

    fn is_filled_source_file(metadata: &fs::Metadata) -> bool {
        !metadata.file_type().is_symlink() && metadata.is_file() && metadata.len() > 0
    }

    pub(super) fn copy_file_out(&self, relative: &Path, destination: &Path) -> std::io::Result<()> {
        let mut source = self.open_regular_file(relative)?;
        let metadata = source.metadata()?;
        Self::require(metadata.len() > 0, "cached artifact is empty")?;
        let mut destination_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        std::io::copy(&mut source, &mut destination_file)?;
        destination_file.flush()?;
        destination_file.sync_all()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(
                destination,
                fs::Permissions::from_mode(cap_std::fs::MetadataExt::mode(&metadata)),
            )?;
        }
        Ok(())
    }

    pub(super) fn remove(&self, relative: &Path) -> std::io::Result<()> {
        let metadata = self.metadata(relative)?;
        Self::require_supported(&metadata, "traversal")?;
        self.children_of(&metadata, relative)?
            .iter()
            .try_for_each(|child| self.remove(&child.relative))?;
        let (parent, name) = self.parent_and_name(relative)?;
        Self::remove_entry(&parent, name, &metadata)
    }

    pub(super) fn path_usage(&self, relative: &Path) -> std::io::Result<(usize, u64)> {
        let metadata = self.metadata(relative)?;
        Self::require_supported(&metadata, "accounting")?;
        let file_bytes = metadata.is_file().then_some(metadata.len());
        let own_usage = (1, file_bytes.unwrap_or_default());
        self.children_of(&metadata, relative)?
            .iter()
            .try_fold(own_usage, |usage, child| {
                self.add_child_usage(usage, &child.relative)
            })
    }

    fn require_supported(metadata: &Metadata, activity: &str) -> std::io::Result<()> {
        if metadata.is_symlink() {
            Err(Self::invalid_data(&format!(
                "cache {activity} encountered a symlink"
            )))
        } else if metadata.is_file() || metadata.is_dir() {
            Ok(())
        } else {
            Err(Self::invalid_data(&format!(
                "cache {activity} encountered an unsupported entry"
            )))
        }
    }

    fn children_of(
        &self,
        metadata: &Metadata,
        relative: &Path,
    ) -> std::io::Result<Vec<CacheEntry>> {
        let children = metadata
            .is_dir()
            .then(|| self.entries(relative))
            .transpose()?;
        Ok(children.unwrap_or_default())
    }

    fn remove_entry(parent: &Dir, name: OsString, metadata: &Metadata) -> std::io::Result<()> {
        if metadata.is_file() {
            parent.remove_file(name)
        } else {
            parent.remove_dir(name)
        }
    }

    fn add_child_usage(
        &self,
        (entries, bytes): (usize, u64),
        child: &Path,
    ) -> std::io::Result<(usize, u64)> {
        let (child_entries, child_bytes) = self.path_usage(child)?;
        let entries = Self::checked_sum_entries([entries, child_entries].into_iter())?;
        let bytes = Self::checked_sum_bytes([bytes, child_bytes].into_iter())?;
        Ok((entries, bytes))
    }

    pub(super) fn prune(&self) -> std::io::Result<()> {
        self.prune_with_limits(Self::MAX_ENTRIES, Self::MAX_BYTES)
    }

    pub(super) fn acquire_lock(
        &self,
        relative: &Path,
        wait: Duration,
    ) -> std::io::Result<CacheLock> {
        self.acquire_lock_with(relative, wait, |file, pid, created, token| {
            writeln!(file, "{pid} {created} {token}")?;
            file.flush()?;
            file.sync_all()
        })
    }

    pub(super) fn acquire_lock_with<F>(
        &self,
        relative: &Path,
        wait: Duration,
        initialize: F,
    ) -> std::io::Result<CacheLock>
    where
        F: Fn(&mut cap_std::fs::File, u32, u64, &str) -> std::io::Result<()>,
    {
        self.acquire_lock_with_runtime(
            relative,
            wait,
            initialize,
            &Self::unix_time,
            &Self::process_is_live,
        )
    }

    fn acquire_lock_with_runtime<F, N, L>(
        &self,
        relative: &Path,
        wait: Duration,
        initialize: F,
        now: &N,
        process_live: &L,
    ) -> std::io::Result<CacheLock>
    where
        F: Fn(&mut cap_std::fs::File, u32, u64, &str) -> std::io::Result<()>,
        N: Fn() -> Duration,
        L: Fn(u32) -> bool,
    {
        let deadline = Instant::now() + wait;
        std::iter::repeat_with(|| {
            self.attempt_lock(relative, &initialize, now, process_live, deadline)
        })
        .find_map(Result::transpose)
        .unwrap_or_else(|| Err(Self::lock_timeout()))
    }

    fn attempt_lock<F, N, L>(
        &self,
        relative: &Path,
        initialize: &F,
        now: &N,
        process_live: &L,
        deadline: Instant,
    ) -> std::io::Result<Option<CacheLock>>
    where
        F: Fn(&mut cap_std::fs::File, u32, u64, &str) -> std::io::Result<()>,
        N: Fn() -> Duration,
        L: Fn(u32) -> bool,
    {
        let created = Self::tolerating(
            self.create_new_file(relative),
            std::io::ErrorKind::AlreadyExists,
        )?;
        created.map_or_else(
            || {
                let recovered = self.recover_lock_with(relative, now, process_live)?;
                Self::retry_lock_after(recovered, deadline).map(|()| None)
            },
            |file| {
                self.initialize_lock(file, relative, initialize, now)
                    .map(Some)
            },
        )
    }

    fn retry_lock_after(recovered: bool, deadline: Instant) -> std::io::Result<()> {
        if recovered {
            Ok(())
        } else if Instant::now() >= deadline {
            Err(Self::lock_timeout())
        } else {
            std::thread::sleep(Self::LOCK_RETRY);
            Ok(())
        }
    }

    fn lock_timeout() -> std::io::Error {
        std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Timed out waiting for handler cache lock",
        )
    }

    fn initialize_lock<F, N>(
        &self,
        mut file: cap_std::fs::File,
        relative: &Path,
        initialize: &F,
        now: &N,
    ) -> std::io::Result<CacheLock>
    where
        F: Fn(&mut cap_std::fs::File, u32, u64, &str) -> std::io::Result<()>,
        N: Fn() -> Duration,
    {
        let identity = EntryIdentity::of(&file.metadata()?);
        let created = now().as_secs();
        let token = format!("{}-{}", std::process::id(), now().as_nanos());
        let mut guard = OwnedCacheEntry {
            cache: self.clone(),
            relative: relative.to_path_buf(),
            identity,
            published: false,
        };
        initialize(&mut file, std::process::id(), created, &token)?;
        guard.publish();
        Ok(CacheLock {
            cache: self.clone(),
            relative: relative.to_path_buf(),
            token,
            identity,
        })
    }

    pub(super) fn prune_with_limits(
        &self,
        max_entries: usize,
        max_bytes: u64,
    ) -> std::io::Result<()> {
        self.prune_with(
            max_entries,
            max_bytes,
            CacheDirectory::remove,
            CacheDirectory::path_usage,
        )
    }

    pub(super) fn prune_with<F, M>(
        &self,
        max_entries: usize,
        max_bytes: u64,
        remove: F,
        measure: M,
    ) -> std::io::Result<()>
    where
        F: Fn(&Self, &Path) -> std::io::Result<()>,
        M: Fn(&Self, &Path) -> std::io::Result<(usize, u64)>,
    {
        self.prune_entries_with_runtime(
            max_entries,
            max_bytes,
            remove,
            measure,
            &Self::unix_time,
            &Self::process_is_live,
        )
    }

    fn prune_entries_with_runtime<F, M, N, L>(
        &self,
        max_entries: usize,
        max_bytes: u64,
        remove: F,
        measure: M,
        now: &N,
        process_live: &L,
    ) -> std::io::Result<()>
    where
        F: Fn(&Self, &Path) -> std::io::Result<()>,
        M: Fn(&Self, &Path) -> std::io::Result<(usize, u64)>,
        N: Fn() -> Duration,
        L: Fn(u32) -> bool,
    {
        self.scavenge_locks_with(now, process_live)?;
        let limits = (max_entries, max_bytes);
        let mut entries = self.prunable_entries(&measure)?;
        entries
            .sort_by(|left, right| (left.modified, &left.name).cmp(&(right.modified, &right.name)));
        let usage = Self::total_usage(&entries)?;
        entries.iter().try_fold(usage, |usage, entry| {
            self.prune_entry(usage, entry, limits, &remove)
        })?;
        let final_usage = Self::total_usage(&self.prunable_entries(&measure)?)?;
        (!Self::exceeds(limits, final_usage))
            .then_some(())
            .ok_or_else(|| {
                std::io::Error::other(
                    "handler cache pruning did not achieve its entry and byte limits",
                )
            })
    }

    fn prune_entry<F>(
        &self,
        usage: (usize, u64),
        entry: &PrunableEntry,
        limits: (usize, u64),
        remove: &F,
    ) -> std::io::Result<(usize, u64)>
    where
        F: Fn(&Self, &Path) -> std::io::Result<()>,
    {
        if !Self::exceeds(limits, usage) {
            return Ok(usage);
        }
        remove(self, &entry.relative)?;
        Ok((
            usage.0.saturating_sub(entry.usage.0),
            usage.1.saturating_sub(entry.usage.1),
        ))
    }

    fn exceeds((max_entries, max_bytes): (usize, u64), (entries, bytes): (usize, u64)) -> bool {
        [entries > max_entries, bytes > max_bytes].contains(&true)
    }

    fn total_usage(entries: &[PrunableEntry]) -> std::io::Result<(usize, u64)> {
        let bytes = Self::checked_sum_bytes(entries.iter().map(|entry| entry.usage.1))?;
        let count = Self::checked_sum_entries(entries.iter().map(|entry| entry.usage.0))?;
        Ok((count, bytes))
    }

    fn scavenge_locks_with<N, L>(&self, now: &N, process_live: &L) -> std::io::Result<()>
    where
        N: Fn() -> Duration,
        L: Fn(u32) -> bool,
    {
        let root = Path::new("");
        self.entries(root)?
            .iter()
            .filter(|entry| Self::is_entry_lock(&entry.name))
            .try_for_each(|entry| {
                self.recover_lock_with(&entry.relative, now, process_live)
                    .map(|_recovered| ())
            })
    }

    fn is_entry_lock(name: &str) -> bool {
        name != ".prune.lock" && name.strip_suffix(".lock").is_some()
    }

    fn prunable_entries<M>(&self, measure: &M) -> std::io::Result<Vec<PrunableEntry>>
    where
        M: Fn(&Self, &Path) -> std::io::Result<(usize, u64)>,
    {
        let root = Path::new("");
        let discovered = self.entries(root)?;
        let active: Vec<String> = discovered
            .iter()
            .filter_map(|entry| Self::active_digest(&entry.name))
            .collect();
        discovered
            .into_iter()
            .filter(|entry| Self::is_prunable_name(&entry.name, &active))
            .map(|entry| self.prunable_entry(entry, measure))
            .collect()
    }

    fn active_digest(name: &str) -> Option<String> {
        name.strip_suffix(".lock")
            .filter(|digest| *digest != ".prune")
            .map(String::from)
    }

    fn is_prunable_name(name: &str, active_digests: &[String]) -> bool {
        !Self::is_lock_file(name) && !Self::belongs_to_active_lock(name, active_digests)
    }

    fn prunable_entry<M>(&self, entry: CacheEntry, measure: &M) -> std::io::Result<PrunableEntry>
    where
        M: Fn(&Self, &Path) -> std::io::Result<(usize, u64)>,
    {
        Self::require(
            !entry.metadata.is_symlink(),
            "cache pruning encountered a symlink",
        )?;
        let modified = entry.metadata.modified()?;
        let usage = measure(self, &entry.relative)?;
        Ok(PrunableEntry {
            relative: entry.relative,
            name: entry.name,
            modified,
            usage,
        })
    }

    fn entries(&self, relative: &Path) -> std::io::Result<Vec<CacheEntry>> {
        self.open_directory(relative)?
            .entries()?
            .map(|entry| self.cache_entry(relative, &entry?))
            .collect()
    }

    fn cache_entry(
        &self,
        relative: &Path,
        entry: &cap_std::fs::DirEntry,
    ) -> std::io::Result<CacheEntry> {
        let name = entry.file_name();
        let name_text = name
            .to_str()
            .ok_or_else(|| Self::invalid_data("cache entry name is not UTF-8"))?
            .to_string();
        let child = relative.join(&name);
        let metadata = self.metadata(&child)?;
        Ok(CacheEntry {
            relative: child,
            name: name_text,
            metadata,
        })
    }

    fn open_regular_file(&self, relative: &Path) -> std::io::Result<cap_std::fs::File> {
        let (parent, name) = self.parent_and_name(relative)?;
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = parent.open_with(name, &options)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.is_symlink() {
            return Err(Self::invalid_data("cache child is not a regular file"));
        }
        Ok(file)
    }

    fn create_new_file(&self, relative: &Path) -> std::io::Result<cap_std::fs::File> {
        let (parent, name) = self.parent_and_name(relative)?;
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        parent.open_with(name, &options)
    }

    fn write_owned_file(&self, relative: &Path, bytes: &[u8]) -> std::io::Result<OwnedCacheEntry> {
        let mut file = self.create_new_file(relative)?;
        let identity = EntryIdentity::of(&file.metadata()?);
        let guard = OwnedCacheEntry {
            cache: self.clone(),
            relative: relative.to_path_buf(),
            identity,
            published: false,
        };
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        Ok(guard)
    }

    fn copy_owned_file(&self, source: &Path, relative: &Path) -> std::io::Result<OwnedCacheEntry> {
        let mut source_file = fs::File::open(source)?;
        let mut destination = self.create_new_file(relative)?;
        let identity = EntryIdentity::of(&destination.metadata()?);
        let guard = OwnedCacheEntry {
            cache: self.clone(),
            relative: relative.to_path_buf(),
            identity,
            published: false,
        };
        std::io::copy(&mut source_file, &mut destination)?;
        destination.flush()?;
        destination.sync_all()?;
        #[cfg(unix)]
        {
            use cap_std::fs::PermissionsExt as _;
            use std::os::unix::fs::PermissionsExt as _;
            destination.set_permissions(cap_std::fs::Permissions::from_mode(
                fs::metadata(source)?.permissions().mode(),
            ))?;
        }
        Ok(guard)
    }

    fn hard_link(&self, source: &Path, destination: &Path) -> std::io::Result<()> {
        let (source_parent, source_name) = self.parent_and_name(source)?;
        let (destination_parent, destination_name) = self.parent_and_name(destination)?;
        source_parent.hard_link(source_name, &destination_parent, destination_name)
    }

    fn rename(&self, source: &Path, destination: &Path) -> std::io::Result<()> {
        let (source_parent, source_name) = self.parent_and_name(source)?;
        let (destination_parent, destination_name) = self.parent_and_name(destination)?;
        source_parent.rename(source_name, &destination_parent, destination_name)
    }

    fn inspect_lock(&self, relative: &Path) -> std::io::Result<LockSnapshot> {
        let mut file = self.open_regular_file(relative)?;
        let metadata = file.metadata()?;
        let identity = EntryIdentity::of(&metadata);
        let modified = metadata
            .modified()?
            .into_std()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let (parser_bytes, overflowed) = LockSnapshot::read_metadata(&mut file)?;
        let fingerprint = LockSnapshot::fingerprint_of(
            identity,
            metadata.len(),
            modified,
            overflowed,
            &parser_bytes,
        );
        let contents = std::str::from_utf8(&parser_bytes)
            .ok()
            .filter(|_| !overflowed)
            .unwrap_or_default();
        Ok(LockSnapshot {
            identity,
            fingerprint,
            owner: LockOwner::parse(contents),
            modified,
        })
    }

    fn lock_is_reclaimable<L>(snapshot: &LockSnapshot, now: Duration, process_live: &L) -> bool
    where
        L: Fn(u32) -> bool,
    {
        let latest_current = now.saturating_add(CacheLock::CLOCK_STEP_TOLERANCE);
        let current = Some(snapshot).filter(|snapshot| snapshot.modified <= latest_current);
        current.is_none_or(|snapshot| Self::lease_is_over(snapshot, now, process_live))
    }

    fn lease_is_over<L>(snapshot: &LockSnapshot, now: Duration, process_live: &L) -> bool
    where
        L: Fn(u32) -> bool,
    {
        snapshot.owner.as_ref().map_or_else(
            || Self::lease_expired(now, snapshot.modified),
            |owner| Self::owner_lease_is_over(owner, snapshot.modified, now, process_live),
        )
    }

    fn owner_lease_is_over<L>(
        owner: &LockOwner,
        modified: Duration,
        now: Duration,
        process_live: &L,
    ) -> bool
    where
        L: Fn(u32) -> bool,
    {
        let created = Duration::from_secs(owner.created);
        let lease_origin = Some(created)
            .filter(|created| *created <= now)
            .unwrap_or(modified);
        !process_live(owner.pid) || Self::lease_expired(now, lease_origin)
    }

    fn lease_expired(now: Duration, lease_origin: Duration) -> bool {
        now.saturating_sub(lease_origin) >= CacheLock::MAX_LEASE
    }

    fn recover_lock_with<N, L>(
        &self,
        relative: &Path,
        now: &N,
        process_live: &L,
    ) -> std::io::Result<bool>
    where
        N: Fn() -> Duration,
        L: Fn(u32) -> bool,
    {
        let before = Self::tolerating(self.inspect_lock(relative), std::io::ErrorKind::NotFound)?;
        before.map_or(Ok(true), |before| {
            let reclaimable = Self::lock_is_reclaimable(&before, now(), process_live);
            let recovered =
                reclaimable.then(|| self.quarantine_lock(relative, &before, now, process_live));
            recovered.unwrap_or(Ok(false))
        })
    }

    fn quarantine_lock<N, L>(
        &self,
        relative: &Path,
        before: &LockSnapshot,
        now: &N,
        process_live: &L,
    ) -> std::io::Result<bool>
    where
        N: Fn() -> Duration,
        L: Fn(u32) -> bool,
    {
        let quarantine = Self::sibling_temporary(relative, "recover")?;
        let moved = Self::tolerating(
            self.rename(relative, &quarantine),
            std::io::ErrorKind::NotFound,
        )?;
        moved.map_or(Ok(true), |()| {
            self.finish_lock_recovery(relative, &quarantine, before, now, process_live)
        })
    }

    fn finish_lock_recovery<N, L>(
        &self,
        relative: &Path,
        quarantine: &Path,
        before: &LockSnapshot,
        now: &N,
        process_live: &L,
    ) -> std::io::Result<bool>
    where
        N: Fn() -> Duration,
        L: Fn(u32) -> bool,
    {
        let after = self.inspect_lock(quarantine)?;
        let removed = Self::is_still_stale(before, &after, now, process_live)
            .then(|| self.remove_unchanged_lock(quarantine, &after))
            .transpose()?
            .unwrap_or(false);
        (!removed)
            .then(|| self.restore_lock(relative, quarantine, &after))
            .transpose()?;
        Ok(removed)
    }

    fn is_still_stale<N, L>(
        before: &LockSnapshot,
        after: &LockSnapshot,
        now: &N,
        process_live: &L,
    ) -> bool
    where
        N: Fn() -> Duration,
        L: Fn(u32) -> bool,
    {
        after.same_lock_as(before) && Self::lock_is_reclaimable(after, now(), process_live)
    }

    fn remove_unchanged_lock(
        &self,
        quarantine: &Path,
        after: &LockSnapshot,
    ) -> std::io::Result<bool> {
        let unchanged = self.inspect_lock(quarantine)?.same_lock_as(after);
        unchanged
            .then(|| self.remove(quarantine))
            .transpose()
            .map(|removed| removed.is_some())
    }

    fn restore_lock(
        &self,
        relative: &Path,
        quarantine: &Path,
        after: &LockSnapshot,
    ) -> std::io::Result<()> {
        let restored = self.hard_link(quarantine, relative).is_ok()
            && self
                .inspect_lock(quarantine)
                .is_ok_and(|current| current.identity == after.identity);
        restored
            .then(|| self.remove(quarantine))
            .transpose()
            .map(|_removed| ())
    }

    fn identity(&self, relative: &Path) -> std::io::Result<EntryIdentity> {
        self.metadata(relative)
            .map(|metadata| EntryIdentity::of(&metadata))
    }

    fn open_directory(&self, relative: &Path) -> std::io::Result<Dir> {
        let mut directory = self.directory.try_clone()?;
        for component in Self::relative_components(relative)? {
            directory = directory.open_dir_nofollow(component)?;
        }
        Ok(directory)
    }

    fn parent_and_name(&self, relative: &Path) -> std::io::Result<(Dir, OsString)> {
        let mut components = Self::relative_components(relative)?;
        let name = components
            .pop()
            .ok_or_else(|| Self::invalid_data("cache operation requires a child path"))?;
        let mut directory = self.directory.try_clone()?;
        for component in components {
            directory = directory.open_dir_nofollow(component)?;
        }
        Ok((directory, name))
    }

    fn is_lock_file(name: &str) -> bool {
        Path::new(name)
            .extension()
            .is_some_and(|extension| extension == "lock")
    }

    fn belongs_to_active_lock(name: &str, active_digests: &[String]) -> bool {
        let name = name.trim_start_matches('.');
        active_digests.iter().any(|digest| name.starts_with(digest))
    }

    fn create_directory_if_missing(parent: &Dir, child: &Path) -> std::io::Result<()> {
        Self::tolerating(parent.create_dir(child), std::io::ErrorKind::AlreadyExists)
            .map(|_created| ())
    }

    fn relative_components(path: &Path) -> std::io::Result<Vec<OsString>> {
        path.components().map(Self::regular_component).collect()
    }

    fn sibling_temporary(path: &Path, purpose: &str) -> std::io::Result<PathBuf> {
        let name = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| Self::invalid_data("cache child name is not UTF-8"))?;
        let temporary = format!(
            ".{name}.{purpose}.{}.{}.tmp",
            std::process::id(),
            Self::unix_time().as_nanos()
        );
        let Some(parent) = path.parent() else {
            return Ok(PathBuf::from(&temporary));
        };
        Ok(parent.join(&temporary))
    }

    fn checked_sum_bytes(values: impl Iterator<Item = u64>) -> std::io::Result<u64> {
        let mut total = 0_u64;
        for value in values {
            total = total
                .checked_add(value)
                .ok_or_else(|| std::io::Error::other("handler cache byte accounting overflow"))?;
        }
        Ok(total)
    }

    fn checked_sum_entries(values: impl Iterator<Item = usize>) -> std::io::Result<usize> {
        let mut total = 0_usize;
        for value in values {
            total = total
                .checked_add(value)
                .ok_or_else(|| std::io::Error::other("handler cache entry accounting overflow"))?;
        }
        Ok(total)
    }

    fn invalid_data(message: &str) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, message)
    }

    fn unix_time() -> Duration {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
    }

    #[cfg(unix)]
    fn process_is_live(pid: u32) -> bool {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    #[cfg(windows)]
    fn process_is_live(pid: u32) -> bool {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
    }
}

impl OwnedCacheEntry {
    pub(super) fn publish(&mut self) {
        self.published = true;
    }

    fn is_unpublished_original(&self) -> bool {
        !self.published && self.cache.identity(&self.relative).ok() == Some(self.identity)
    }
}

impl Drop for OwnedCacheEntry {
    fn drop(&mut self) {
        let _removed = self
            .is_unpublished_original()
            .then(|| self.cache.remove(&self.relative));
    }
}

impl CacheLock {
    const MAX_METADATA_BYTES: usize = 1024;
    const MAX_LEASE: Duration = Duration::from_mins(10);
    /// How far a lock may look newer than the clock and still be current. The wall clock can
    /// step back while a lock is held (virtual machines resynchronise with their host every few
    /// seconds), and treating every newer lock as forged would hand a live lock to a second
    /// holder.
    const CLOCK_STEP_TOLERANCE: Duration = Duration::from_mins(1);

    fn is_still_held(&self) -> bool {
        self.cache
            .inspect_lock(&self.relative)
            .is_ok_and(|snapshot| self.holds(&snapshot))
    }

    fn holds(&self, snapshot: &LockSnapshot) -> bool {
        snapshot.identity == self.identity
            && snapshot
                .owner
                .as_ref()
                .is_some_and(|owner| owner.token == self.token)
    }
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        let _removed = self
            .is_still_held()
            .then(|| self.cache.remove(&self.relative));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use std::cell::Cell;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    fn seed(cache: &CacheDirectory, relative: &str, bytes: &[u8]) {
        let mut entry = cache
            .write_owned_file(Path::new(relative), bytes)
            .expect("seed cache entry");
        entry.publish();
    }

    fn set_modified(root: &Path, relative: &str, modified: Duration) {
        fs::OpenOptions::new()
            .write(true)
            .open(root.join(relative))
            .expect("open cache entry for timestamp")
            .set_modified(SystemTime::UNIX_EPOCH + modified)
            .expect("set cache entry timestamp");
    }

    fn assert_future_lock_digest_is_reclaimed(
        cache: &CacheDirectory,
        now: Duration,
        process_live: impl Fn(u32) -> bool,
    ) {
        seed(cache, ".future.publish.tmp", b"temporary");
        seed(cache, "future.bin", b"artifact");
        cache
            .prune_entries_with_runtime(
                0,
                0,
                CacheDirectory::remove,
                CacheDirectory::path_usage,
                &|| now,
                &process_live,
            )
            .expect("future lock and associated state are reclaimable");
        assert!(cache.metadata(Path::new("future.lock")).is_err());
        assert!(cache.metadata(Path::new(".future.publish.tmp")).is_err());
        assert!(cache.metadata(Path::new("future.bin")).is_err());
        assert!(
            cache
                .prunable_entries(&CacheDirectory::path_usage)
                .expect("postcondition entries")
                .is_empty()
        );
    }

    #[cfg(unix)]
    fn publish_compiled_artifact(cache: &CacheDirectory, destination: &Path) {
        let compiler = tempfile::tempdir().expect("compiler workspace");
        let output = compiler.path().join("artifact");
        fs::write(&output, b"compiled").expect("compiler output");
        cache
            .publish_file(&output, destination)
            .expect("publish through opened directory");
    }

    #[cfg(unix)]
    #[test]
    fn opened_cache_capability_survives_ambient_root_swap() {
        let project = tempfile::tempdir().expect("project");
        let cache = CacheDirectory::open(project.path()).expect("open cache capability");
        let ambient = project.path().join(".tachyon/handlers");
        fs::write(ambient.join("stale"), b"obsolete").expect("seed prune candidate");

        let owned = project.path().join(".tachyon/handlers-owned");
        fs::rename(&ambient, &owned).expect("move opened cache directory");
        let outside = tempfile::tempdir().expect("outside directory");
        let sentinel = outside.path().join("sentinel");
        fs::write(&sentinel, b"outside").expect("outside sentinel");
        symlink(outside.path(), &ambient).expect("replace ambient cache with symlink");

        let lock = cache
            .acquire_lock(Path::new("capability.lock"), Duration::from_secs(1))
            .expect("lock through opened directory");
        assert!(owned.join("capability.lock").is_file());
        assert!(!outside.path().join("capability.lock").exists());
        drop(lock);

        cache
            .prune_with_limits(0, 0)
            .expect("prune through opened directory");
        assert!(!owned.join("stale").exists());

        cache
            .stage_bytes(Path::new("stage.bin"), b"staged")
            .expect("stage through opened directory");
        publish_compiled_artifact(&cache, Path::new("artifact.bin"));

        assert!(cache.is_valid_file(Path::new("stage.bin")));
        assert_eq!(
            cache
                .read(Path::new("artifact.bin"))
                .expect("discover artifact"),
            b"compiled"
        );
        assert_eq!(fs::read(&sentinel).expect("read sentinel"), b"outside");
        assert_eq!(
            fs::read_dir(outside.path())
                .expect("outside listing")
                .count(),
            1
        );
        assert_eq!(
            fs::read(owned.join("stage.bin")).expect("owned stage"),
            b"staged"
        );
        assert_eq!(
            fs::read(owned.join("artifact.bin")).expect("owned artifact"),
            b"compiled"
        );
    }

    #[test]
    fn pruning_scavenges_a_fresh_dead_lock_for_an_abandoned_digest() {
        let root = tempfile::tempdir().expect("cache");
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        seed(&cache, "old-digest.lock", b"41 999 dead-token\n");
        seed(&cache, "old-digest.bin", b"abandoned artifact");

        cache
            .prune_entries_with_runtime(
                0,
                0,
                CacheDirectory::remove,
                CacheDirectory::path_usage,
                &|| Duration::from_secs(1_000),
                &|pid| {
                    assert_eq!(pid, 41);
                    false
                },
            )
            .expect("scavenge crashed build after source digest changed");

        assert!(cache.metadata(Path::new("old-digest.lock")).is_err());
        assert!(cache.metadata(Path::new("old-digest.bin")).is_err());
    }

    #[test]
    fn fresh_dead_lock_is_recovered_before_the_acquisition_deadline() {
        let root = tempfile::tempdir().expect("cache");
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        seed(&cache, "fresh-dead.lock", b"47 999 dead-token\n");
        let now = || Duration::from_secs(1_000);

        let recovered = cache
            .acquire_lock_with_runtime(
                Path::new("fresh-dead.lock"),
                Duration::from_secs(1),
                |file, pid, created, token| writeln!(file, "{pid} {created} {token}"),
                &now,
                &|pid| {
                    assert_eq!(pid, 47);
                    false
                },
            )
            .expect("fresh dead process recovers immediately");
        assert_ne!(
            cache
                .read(Path::new("fresh-dead.lock"))
                .expect("replacement lock"),
            b"47 999 dead-token\n"
        );
        drop(recovered);
    }

    #[test]
    fn live_lock_is_not_stolen_within_a_legitimate_transaction() {
        let root = tempfile::tempdir().expect("cache");
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        seed(&cache, "live.lock", b"42 1 live-token\n");
        set_modified(root.path(), "live.lock", Duration::from_secs(1));
        let now = || Duration::from_mins(1);
        let live = |pid| pid == 42;

        let failure = cache
            .acquire_lock_with_runtime(
                Path::new("live.lock"),
                Duration::ZERO,
                |_, _, _, _| Ok(()),
                &now,
                &live,
            )
            .expect_err("live transaction must retain its lock");
        assert_eq!(failure.kind(), std::io::ErrorKind::TimedOut);
        assert_eq!(
            cache.read(Path::new("live.lock")).expect("live lock"),
            b"42 1 live-token\n"
        );
        assert!(CacheLock::MAX_LEASE > Duration::from_mins(1));
    }

    #[test]
    fn hard_lease_bounds_a_lock_whose_pid_has_been_reused() {
        let root = tempfile::tempdir().expect("cache");
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        seed(&cache, "reused.lock", b"43 100 reused-token\n");
        let now = || Duration::from_secs(100) + CacheLock::MAX_LEASE;

        let recovered = cache
            .acquire_lock_with_runtime(
                Path::new("reused.lock"),
                Duration::from_millis(20),
                |file, pid, created, token| writeln!(file, "{pid} {created} {token}"),
                &now,
                &|pid| {
                    assert_eq!(pid, 43);
                    true
                },
            )
            .expect("hard lease recovers PID-reuse lock");
        assert_ne!(
            cache.read(Path::new("reused.lock")).expect("new lock"),
            b"43 100 reused-token\n"
        );
        drop(recovered);
    }

    #[test]
    fn future_owner_timestamp_falls_back_to_the_metadata_lease() {
        let snapshot = LockSnapshot {
            identity: EntryIdentity {
                device: 1,
                inode: 2,
            },
            fingerprint: [0; 32],
            owner: Some(LockOwner {
                pid: 43,
                created: 10_000,
                token: "future-token".to_string(),
            }),
            modified: Duration::from_secs(100),
        };
        let live = |pid| {
            assert_eq!(pid, 43);
            true
        };

        assert!(!CacheDirectory::lock_is_reclaimable(
            &snapshot,
            (Duration::from_secs(100) + CacheLock::MAX_LEASE)
                .checked_sub(Duration::from_nanos(1))
                .expect("positive lease"),
            &live,
        ));
        assert!(CacheDirectory::lock_is_reclaimable(
            &snapshot,
            Duration::from_secs(100) + CacheLock::MAX_LEASE,
            &live,
        ));
    }

    #[test]
    fn malformed_lock_with_future_mtime_cannot_escape_quota() {
        let root = tempfile::tempdir().expect("cache");
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        seed(&cache, "future.lock", b"malformed");
        set_modified(root.path(), "future.lock", Duration::from_secs(1_000));

        assert_future_lock_digest_is_reclaimed(&cache, Duration::from_secs(100), |_| true);
    }

    #[test]
    fn live_future_owner_and_future_mtime_cannot_escape_quota() {
        let root = tempfile::tempdir().expect("cache");
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        seed(&cache, "future.lock", b"43 10000 future-token\n");
        set_modified(root.path(), "future.lock", Duration::from_secs(1_000));

        assert_future_lock_digest_is_reclaimed(&cache, Duration::from_secs(100), |pid| {
            assert_eq!(pid, 43);
            true
        });
    }

    #[test]
    fn a_live_lock_survives_the_clock_stepping_back() {
        let root = tempfile::tempdir().expect("cache");
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        let lock = Path::new("digest.lock");
        let held = cache
            .acquire_lock(lock, Duration::from_secs(1))
            .expect("first holder");
        let modified = cache.inspect_lock(lock).expect("held lock").modified;
        let stepped_back = || {
            modified
                .checked_sub(Duration::from_millis(250))
                .expect("positive time")
        };
        let live = |_| true;

        let competing = cache.acquire_lock_with_runtime(
            lock,
            Duration::from_millis(50),
            |file, pid, created, token| writeln!(file, "{pid} {created} {token}"),
            &stepped_back,
            &live,
        );
        assert!(competing.is_err(), "a second holder took a live lock");
        cache
            .prune_entries_with_runtime(
                CacheDirectory::MAX_ENTRIES,
                CacheDirectory::MAX_BYTES,
                CacheDirectory::remove,
                CacheDirectory::path_usage,
                &stepped_back,
                &live,
            )
            .expect("prune with a stepped clock");
        assert!(held.is_still_held(), "pruning reclaimed a live lock");
        drop(held);
    }

    #[test]
    fn oversized_sparse_lock_is_inspected_and_recovered_with_bounded_io() {
        let root = tempfile::tempdir().expect("cache");
        let path = root.path().join("oversized.lock");
        let file = fs::File::create(&path).expect("create sparse lock");
        file.set_len((CacheLock::MAX_METADATA_BYTES + 1) as u64)
            .expect("make oversized lock");
        drop(file);
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        let now = || Duration::from_secs(10_000_000_000);

        let snapshot = cache
            .inspect_lock(Path::new("oversized.lock"))
            .expect("bounded inspection");
        assert!(snapshot.owner.is_none(), "oversized metadata is malformed");
        let recovered = cache
            .acquire_lock_with_runtime(
                Path::new("oversized.lock"),
                Duration::from_millis(20),
                |file, pid, created, token| writeln!(file, "{pid} {created} {token}"),
                &now,
                &|_| true,
            )
            .expect("expired oversized lock is recovered");
        assert!(
            cache
                .metadata(Path::new("oversized.lock"))
                .expect("replacement metadata")
                .len()
                < CacheLock::MAX_METADATA_BYTES as u64
        );
        drop(recovered);
    }

    #[test]
    fn oversized_sparse_lock_with_future_mtime_cannot_escape_quota() {
        let root = tempfile::tempdir().expect("cache");
        let path = root.path().join("future.lock");
        let file = fs::File::create(&path).expect("create sparse lock");
        file.set_len((CacheLock::MAX_METADATA_BYTES + 1) as u64)
            .expect("make oversized lock");
        file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000))
            .expect("set hostile future mtime");
        drop(file);
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");

        assert_future_lock_digest_is_reclaimed(&cache, Duration::from_secs(100), |_| true);
    }

    #[test]
    fn abandoned_temporaries_are_accounted_and_quota_is_a_postcondition() {
        let root = tempfile::tempdir().expect("cache");
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        seed(&cache, "dead.lock", b"44 499 dead-token\n");
        seed(&cache, ".dead.publish.tmp", b"temporary");
        seed(&cache, "dead.bin", b"artifact");
        seed(&cache, "survivor", b"ok");
        let now = || Duration::from_secs(500);
        let dead = |_| false;

        cache
            .prune_entries_with_runtime(
                1,
                2,
                CacheDirectory::remove,
                CacheDirectory::path_usage,
                &now,
                &dead,
            )
            .expect("enforce quota after scavenging");
        assert!(cache.metadata(Path::new("dead.lock")).is_err());
        assert!(cache.metadata(Path::new(".dead.publish.tmp")).is_err());
        assert!(cache.metadata(Path::new("dead.bin")).is_err());
        let remaining = cache
            .prunable_entries(&CacheDirectory::path_usage)
            .expect("remaining entries");
        assert!(
            CacheDirectory::checked_sum_entries(remaining.iter().map(|entry| entry.usage.0))
                .expect("entry accounting")
                <= 1
        );
        assert!(
            CacheDirectory::checked_sum_bytes(remaining.iter().map(|entry| entry.usage.1))
                .expect("byte accounting")
                <= 2
        );
    }

    #[test]
    fn pruning_rejects_a_false_success_that_does_not_achieve_quota() {
        let root = tempfile::tempdir().expect("cache");
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        seed(&cache, "entry", b"too large");
        let failure = cache
            .prune_entries_with_runtime(
                0,
                0,
                |_, _| Ok(()),
                CacheDirectory::path_usage,
                &|| Duration::from_secs(1),
                &|_| false,
            )
            .expect_err("reported removal did not achieve quota");
        assert!(failure.to_string().contains("did not achieve"));
    }

    #[test]
    fn quarantine_recovery_never_deletes_a_replacement_lock() {
        let root = tempfile::tempdir().expect("cache");
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        seed(&cache, "replace.lock", b"45 1 old-token\n");
        set_modified(root.path(), "replace.lock", Duration::from_secs(1));
        let observations = Cell::new(0_u8);
        let live = |pid| {
            assert_eq!(pid, 45);
            observations.set(observations.get() + 1);
            if observations.get() == 2 {
                let quarantine = fs::read_dir(root.path())
                    .expect("cache listing")
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .find(|path| {
                        path.file_name()
                            .and_then(OsStr::to_str)
                            .is_some_and(|name| name.contains(".recover."))
                    })
                    .expect("quarantined lock");
                fs::remove_file(&quarantine).expect("replace quarantine");
                fs::write(&quarantine, b"46 999 replacement-token\n").expect("replacement lock");
            }
            false
        };

        assert!(
            !cache
                .recover_lock_with(Path::new("replace.lock"), &|| Duration::from_secs(2), &live,)
                .expect("safe recovery")
        );
        assert_eq!(
            cache
                .read(Path::new("replace.lock"))
                .expect("replacement survives"),
            b"46 999 replacement-token\n"
        );
    }
}
