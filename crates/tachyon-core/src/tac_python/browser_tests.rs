use super::runtime_assets::RuntimeAssets;
use crate::{BuildOptions, BuildProfile, WebCompiler};
use std::path::{Path, PathBuf};
use std::time::Duration;

struct PythonTacFixture {
    root: PathBuf,
    core: PathBuf,
    binary: PathBuf,
    repo: PathBuf,
    source: String,
}

impl PythonTacFixture {
    const FILES: [(&'static str, &'static str); 11] = [
        (
            "client/helpers/ShelfHelper.py",
            include_str!("../../tests/fixtures/tac-python/client/helpers/ShelfHelper.py"),
        ),
        (
            "client/services/LibraryService.py",
            include_str!("../../tests/fixtures/tac-python/client/services/LibraryService.py"),
        ),
        (
            "client/models/Guide.py",
            include_str!("../../tests/fixtures/tac-python/client/models/Guide.py"),
        ),
        (
            "client/helpers/AuditHelper.py",
            include_str!("../../tests/fixtures/tac-python/client/helpers/AuditHelper.py"),
        ),
        (
            "client/helpers/CounterHelper.py",
            include_str!("../../tests/fixtures/tac-python/client/helpers/CounterHelper.py"),
        ),
        (
            "client/services/AuditService.py",
            include_str!("../../tests/fixtures/tac-python/client/services/AuditService.py"),
        ),
        (
            "client/services/CounterService.py",
            include_str!("../../tests/fixtures/tac-python/client/services/CounterService.py"),
        ),
        (
            "client/pages/tac.html",
            include_str!("../../tests/fixtures/tac-python/client/pages/tac.html"),
        ),
        (
            "client/components/python/probe/tac.html",
            include_str!("../../tests/fixtures/tac-python/client/components/python/probe/tac.html"),
        ),
        (
            "client/components/python/lifecycle/tac.html",
            include_str!(
                "../../tests/fixtures/tac-python/client/components/python/lifecycle/tac.html"
            ),
        ),
        (
            "client/manifest.json",
            include_str!("../../tests/fixtures/tac-python/client/manifest.json"),
        ),
    ];

    async fn run() {
        let fixture = Self::read();
        fixture.require_local_cache();
        fixture.stage();
        fixture.stage_assets();
        fixture.build().await;
        fixture.certificate().await;
        fixture.browser().await;
    }

    fn read() -> Self {
        Self {
            root: PathBuf::from(Self::required("TAC_PYTHON_FIXTURE_OUT")),
            core: PathBuf::from(Self::required("TAC_PYTHON_CORE")),
            binary: PathBuf::from(Self::required("TAC_BIN")),
            repo: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            source: Self::required("TAC_PYTHON_SOURCE_COMMIT")
                .to_string_lossy()
                .into_owned(),
        }
    }

    fn required(name: &str) -> std::ffi::OsString {
        std::env::var_os(name)
            .unwrap_or_else(|| panic!("{name} must explicitly identify the retained local fixture"))
    }

    fn require_local_cache(&self) {
        assert!(self.root.is_absolute(), "fixture root must be absolute");
        assert_eq!(
            crate::cache::root(),
            self.root.join("cache"),
            "set TAC_CACHE_DIR to the explicit fixture's cache; the probe never writes the global cache"
        );
        assert!(self.binary.is_file(), "explicit preview binary must exist");
    }

    fn stage(&self) {
        for (path, source) in Self::FILES {
            Self::write(&self.root.join(path), source.as_bytes());
        }
    }

    fn stage_assets(&self) {
        let verified = RuntimeAssets::read(&self.core)
            .unwrap_or_else(|error| panic!("pinned fixture assets: {error:?}"));
        let target = RuntimeAssets::cached_root();
        for (path, bytes) in verified {
            let name = Path::new(&path)
                .file_name()
                .unwrap_or_else(|| panic!("pinned filename"));
            Self::write(&target.join(name), &bytes);
        }
    }

    async fn build(&self) {
        let options = BuildOptions {
            output_directory: PathBuf::from("dist"),
            incremental: false,
            profile: BuildProfile::Release,
        };
        let built = WebCompiler::build_async(&self.root, &options)
            .await
            .unwrap_or_else(|error| panic!("ordinary compiler build: {error:?}"));
        let receipt = serde_json::json!({"sourceCommit": self.source, "buildSha256": built.sha256(), "runtime": RuntimeAssets::RUNTIME, "scope": "Python Tac fixture: synchronous and async def services; runtime qualification follows"});
        Self::write(
            &self.root.join("python-tac-build.json"),
            receipt.to_string().as_bytes(),
        );
    }

    async fn certificate(&self) {
        assert!(self.root.is_absolute(), "fixture root must be absolute");
        let mut command = tokio::process::Command::new("openssl");
        command
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=python-tac.test",
                "-addext",
                "subjectAltName=DNS:python-tac.test",
                "-keyout",
            ])
            .arg(self.root.join("python-tac-test-key.pem"))
            .arg("-out")
            .arg(self.root.join("python-tac-test-cert.pem"));
        let output = crate::external_command::run(&mut command, Duration::from_secs(10), 64 * 1024)
            .await
            .unwrap_or_else(|error| panic!("supervised fixture certificate: {error}"));
        assert!(
            output.status.success(),
            "fixture certificate: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    async fn browser(&self) {
        let mut command = crate::node_driver::NodeDriver::async_command();
        command
            .arg(self.repo.join("scripts/python-tac-browser-test.mjs"))
            .current_dir(&self.repo)
            .env("TAC_PYTHON_FIXTURE_OUT", &self.root)
            .env("TAC_PYTHON_SOURCE_COMMIT", &self.source)
            .env("TAC_BIN", &self.binary);
        let output =
            crate::node_driver::NodeDriver::run_within(&mut command, Duration::from_mins(16))
                .await
                .unwrap_or_else(|error| panic!("supervised browser probe: {error}"));
        assert!(
            output.status.success(),
            "browser probe: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }

    fn write(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap_or_else(|| panic!("fixture parent")))
            .unwrap_or_else(|error| panic!("fixture directory: {error}"));
        std::fs::write(path, bytes).unwrap_or_else(|error| panic!("fixture source: {error}"));
    }
}

#[tokio::test]
#[ignore = "requires explicit pinned core assets, retained fixture/cache, source commit, preview binary and Playwright Chromium, Firefox and WebKit"]
async fn pinned_python_tac_services_async_models_and_offline_reload_use_the_real_compiler() {
    PythonTacFixture::run().await;
}

#[tokio::test]
#[ignore = "requires the explicit retained Python Tac fixture environment; issues only a local one-day test certificate"]
async fn python_tac_fixture_certificate_uses_the_supervised_sdk_path() {
    PythonTacFixture::read().certificate().await;
}
