use std::path::{Path, PathBuf};
use std::time::Duration;

pub(super) struct OrdinaryBrowserFixture {
    pub(super) root: PathBuf,
    pub(super) repo: PathBuf,
    binary: PathBuf,
    source: std::ffi::OsString,
}

impl OrdinaryBrowserFixture {
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

    fn write(&self, files: impl IntoIterator<Item = (&'static str, &'static str)>) {
        for (relative, source) in files {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap_or_else(|| panic!("fixture parent")))
                .unwrap_or_else(|error| panic!("fixture directory: {error}"));
            std::fs::write(path, source).unwrap_or_else(|error| panic!("fixture source: {error}"));
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
