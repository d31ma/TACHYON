use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _, OpenOptionsSyncExt as _};
use cap_std::fs::{Dir, File, OpenOptions};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(super) struct OrdinaryBrowserFixture {
    pub(super) root: PathBuf,
    pub(super) repo: PathBuf,
    binary: PathBuf,
    source: std::ffi::OsString,
}

impl OrdinaryBrowserFixture {
    const SOURCE_LIMIT: u64 = 1024 * 1024;

    pub(super) fn read(names: [&str; 3]) -> Self {
        Self {
            root: PathBuf::from(Self::required(names[0])),
            binary: PathBuf::from(Self::required(names[1])),
            repo: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            source: Self::required(names[2]),
        }
    }

    pub(super) fn validate(&self) {
        assert!(self.root.is_absolute(), "fixture root must be absolute");
        assert!(self.binary.is_file(), "explicit compiler must exist");
        assert_eq!(
            crate::cache::root(),
            self.root.join("cache"),
            "TAC_CACHE_DIR must select the fixture's isolated cache"
        );
    }

    pub(super) fn stage(&self, files: impl IntoIterator<Item = (&'static str, &'static str)>) {
        self.write(files);
    }

    pub(super) fn stage_repo_files(&self, source_root: &str, files: &[&str]) {
        self.write(Self::repo_sources(source_root, files));
    }

    pub(super) fn repo_source(source_root: &str, relative: &str) -> String {
        Self::source_text(Path::new(source_root), Path::new(relative))
    }

    fn repo_sources<'a>(source_root: &str, files: &'a [&'a str]) -> Vec<(&'a str, String)> {
        assert!(files.len() <= 32, "fixture source list must remain bounded");
        files
            .iter()
            .map(|relative| (*relative, Self::repo_source(source_root, relative)))
            .collect()
    }

    fn source_text(source_root: &Path, relative: &Path) -> String {
        Self::require_source_path(source_root);
        Self::require_source_path(relative);
        let directory = Self::source_directory(source_root, relative);
        let name = relative
            .file_name()
            .unwrap_or_else(|| panic!("fixture source needs a filename"));
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No).nonblock(true);
        let file = directory
            .open_with(name, &options)
            .unwrap_or_else(|error| panic!("regular fixture source: {error}"));
        Self::read_source(file)
    }

    fn require_source_path(path: &Path) {
        assert!(
            !path.as_os_str().is_empty(),
            "fixture source path is required"
        );
        assert!(
            path.components().count() <= 16,
            "fixture source path is too deep"
        );
        assert!(
            path.components()
                .all(|component| matches!(component, std::path::Component::Normal(_))),
            "fixture source paths must be relative without dot segments"
        );
    }

    fn source_directory(source_root: &Path, relative: &Path) -> Dir {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| panic!("fixture repository root"));
        let parent = repository
            .parent()
            .unwrap_or_else(|| panic!("fixture repository parent"));
        let name = repository
            .file_name()
            .unwrap_or_else(|| panic!("fixture repository name"));
        let ambient = Dir::open_ambient_dir(parent, cap_std::ambient_authority())
            .unwrap_or_else(|error| panic!("fixture repository parent: {error}"));
        let mut directory = ambient
            .open_dir_nofollow(name)
            .unwrap_or_else(|error| panic!("regular fixture repository: {error}"));
        let folders = source_root.join(relative.parent().unwrap_or_else(|| Path::new("")));
        for component in folders.components() {
            directory = directory
                .open_dir_nofollow(component.as_os_str())
                .unwrap_or_else(|error| panic!("regular fixture source directory: {error}"));
        }
        directory
    }

    fn read_source(file: File) -> String {
        let metadata = file
            .metadata()
            .unwrap_or_else(|error| panic!("fixture source metadata: {error}"));
        assert!(metadata.is_file(), "fixture source must be a regular file");
        assert!(
            metadata.len() <= Self::SOURCE_LIMIT,
            "fixture source exceeds 1 MiB"
        );
        let mut bytes = Vec::new();
        file.take(Self::SOURCE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .unwrap_or_else(|error| panic!("bounded fixture source read: {error}"));
        assert_eq!(
            bytes.len() as u64,
            metadata.len(),
            "fixture source changed length while captured"
        );
        String::from_utf8(bytes).unwrap_or_else(|error| panic!("UTF-8 fixture source: {error}"))
    }

    fn write<'a, T: AsRef<str>>(&self, files: impl IntoIterator<Item = (&'a str, T)>) {
        for (relative, source) in files {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap_or_else(|| panic!("fixture parent")))
                .unwrap_or_else(|error| panic!("fixture directory: {error}"));
            std::fs::write(path, source.as_ref())
                .unwrap_or_else(|error| panic!("fixture source: {error}"));
        }
    }

    pub(super) async fn build(&self) {
        let mut command = tokio::process::Command::new(&self.binary);
        command
            .arg("bundle")
            .arg(&self.root)
            .args(["--out-dir", "dist", "--no-incremental"])
            .current_dir(&self.repo);
        Self::execute(&mut command, Duration::from_mins(6)).await;
    }

    pub(super) async fn browser(&self, names: [&str; 3], script: &str) {
        let mut command = tokio::process::Command::new("node");
        command
            .arg(self.repo.join(script))
            .env(names[1], &self.binary)
            .env(names[0], &self.root)
            .env(names[2], &self.source)
            .current_dir(&self.repo);
        Self::execute(&mut command, Duration::from_mins(1)).await;
    }

    pub(super) async fn certificate(&self, host: &str, files: [&str; 2]) {
        let mut command = tokio::process::Command::new("openssl");
        command
            .args([
                "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
            ])
            .args(["-subj", &format!("/CN={host}")])
            .args(["-addext", &format!("subjectAltName=DNS:{host}")])
            .arg("-keyout")
            .arg(self.root.join(files[0]))
            .arg("-out")
            .arg(self.root.join(files[1]));
        Self::execute_bounded(&mut command, Duration::from_secs(10), 64 * 1024).await;
    }

    fn required(name: &str) -> std::ffi::OsString {
        std::env::var_os(name)
            .unwrap_or_else(|| panic!("{name} must identify this explicit opt-in fixture"))
    }

    async fn execute(command: &mut tokio::process::Command, deadline: Duration) {
        Self::execute_bounded(command, deadline, 262_144).await;
    }

    async fn execute_bounded(
        command: &mut tokio::process::Command,
        deadline: Duration,
        output_limit: usize,
    ) {
        let output = crate::external_command::run(command, deadline, output_limit)
            .await
            .unwrap_or_else(|error| panic!("supervised ordinary browser fixture: {error}"));
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
}
