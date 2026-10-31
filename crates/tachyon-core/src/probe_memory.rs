use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Remembers, for one nextest run, which programs answered a probe, so each test process does not
/// start the same compiler again only to learn it is installed. Only answers that succeeded are
/// remembered: a probe that fails is run again by the next test that needs it.
pub(crate) struct ProbeMemory {
    folder: Option<PathBuf>,
}

impl ProbeMemory {
    const RUN_VARIABLE: &'static str = "NEXTEST_RUN_ID";
    const FOLDER: &'static str = "tachyon-test-probes";
    const ANSWERED: &'static str = "answered";
    const OLDEST_RUN: Duration = Duration::from_secs(24 * 60 * 60);

    pub(crate) fn for_this_run() -> Self {
        let folder = Self::run_folder();
        Self { folder }
    }

    pub(crate) fn answered(&self, program: &str, flag: &str) -> bool {
        let entry = self.entry(program, flag);
        let text = entry.and_then(|path| std::fs::read_to_string(path).ok());
        text.as_deref() == Some(Self::ANSWERED)
    }

    pub(crate) fn remember(&self, program: &str, flag: &str, answered: bool) -> bool {
        let entry = self.entry(program, flag).filter(|_| answered);
        Self::write(entry);
        answered
    }

    fn run_folder() -> Option<PathBuf> {
        let run = std::env::var(Self::RUN_VARIABLE).ok()?;
        let runs = std::env::temp_dir().join(Self::FOLDER);
        let folder = runs.join(run);
        std::fs::create_dir_all(&folder).ok()?;
        Self::forget_old_runs(&runs, &folder);
        Some(folder)
    }

    fn entry(&self, program: &str, flag: &str) -> Option<PathBuf> {
        let folder = self.folder.as_ref()?;
        let mut hasher = DefaultHasher::new();
        program.hash(&mut hasher);
        flag.hash(&mut hasher);
        std::env::var_os("PATH").hash(&mut hasher);
        Some(folder.join(format!("{:016x}", hasher.finish())))
    }

    fn write(entry: Option<PathBuf>) {
        let Some(path) = entry else {
            return;
        };
        let pending = path.with_extension(format!("pending-{}", std::process::id()));
        let written = std::fs::write(&pending, Self::ANSWERED);
        let _ = written.and_then(|()| std::fs::rename(&pending, &path));
        let _ = std::fs::remove_file(&pending);
    }

    fn forget_old_runs(runs: &Path, current: &Path) {
        let listing = std::fs::read_dir(runs).into_iter().flatten().flatten();
        listing
            .map(|entry| entry.path())
            .filter(|path| Self::is_old_run(path, current))
            .for_each(|path| Self::forget(&path));
    }

    fn is_old_run(path: &Path, current: &Path) -> bool {
        let modified = std::fs::metadata(path).and_then(|metadata| metadata.modified());
        let age = modified.map(|time| SystemTime::now().duration_since(time).unwrap_or_default());
        path != current && age.is_ok_and(|age| age > Self::OLDEST_RUN)
    }

    fn forget(path: &Path) {
        let _ = std::fs::remove_dir_all(path);
    }
}
