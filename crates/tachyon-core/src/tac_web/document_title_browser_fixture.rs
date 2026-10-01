use std::path::{Path, PathBuf};
use std::time::Duration;

struct DocumentTitleBrowserFixture {
    root: PathBuf,
    binary: PathBuf,
}

impl DocumentTitleBrowserFixture {
    const PATHS: [&'static str; 8] = [
        "client/manifest.json",
        "client/pages/tac.html",
        "client/services/HarnessService.js",
        "client/services/KotlinTitleService.kt",
        "client/services/ScriptTitleService.ts",
        "client/components/title/harness/tac.html",
        "client/components/title/kotlin/tac.html",
        "client/components/title/script/tac.html",
    ];

    async fn run() {
        let fixture = Self {
            root: Self::required_path("TAC_DOCUMENT_TITLE_FIXTURE_OUT"),
            binary: Self::required_path("TAC_BIN"),
        };
        assert!(fixture.root.is_absolute(), "fixture root must be absolute");
        assert!(
            fixture.binary.is_file(),
            "explicit compiler binary must exist"
        );
        assert!(
            std::env::var_os("TAC_DOCUMENT_TITLE_SOURCE_COMMIT").is_some(),
            "compiler source commit must be explicit"
        );
        assert_eq!(
            crate::cache::root(),
            fixture.root.join("cache"),
            "fixture cache must be isolated"
        );
        fixture.stage();
        Self::execute(fixture.bundle_command(), Duration::from_mins(6)).await;
        Self::execute(fixture.browser_command(), Duration::from_mins(1)).await;
    }

    fn required_path(name: &str) -> PathBuf {
        let value = std::env::var_os(name).expect("explicit DocumentTitle fixture environment");
        PathBuf::from(value)
    }

    fn repository() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn stage(&self) {
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tac-document-title");
        for relative in Self::PATHS {
            let target = self.root.join(relative);
            std::fs::create_dir_all(target.parent().expect("literal fixture parent"))
                .expect("fixture directory creation");
            std::fs::copy(source.join(relative), target)
                .expect("retained DocumentTitle source copy");
        }
    }

    fn bundle_command(&self) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(&self.binary);
        command
            .args(["bundle"])
            .arg(&self.root)
            .args(["--out-dir", "dist", "--no-incremental"]);
        command.current_dir(Self::repository());
        command
    }

    fn browser_command(&self) -> tokio::process::Command {
        let mut command = tokio::process::Command::new("node");
        command.arg(Self::repository().join("scripts/document-title-browser-gate.mjs"));
        command
            .env("TAC_BIN", &self.binary)
            .env("TAC_DOCUMENT_TITLE_FIXTURE_OUT", &self.root);
        command.current_dir(Self::repository());
        command
    }

    async fn execute(mut command: tokio::process::Command, deadline: Duration) {
        let captured = crate::external_command::run(&mut command, deadline, 262_144)
            .await
            .expect("supervised DocumentTitle fixture command");
        let stdout = String::from_utf8_lossy(&captured.stdout);
        let stderr = String::from_utf8_lossy(&captured.stderr);
        assert!(
            captured.status.success(),
            "DocumentTitle fixture command failed:\n{stdout}\n{stderr}"
        );
        println!("{stdout}");
    }
}

#[tokio::test]
#[ignore = "requires explicit current DocumentTitle compiler, isolated fixture/cache, Kotlin/Wasm SDK and installed Chromium"]
async fn document_title_uses_the_ordinary_kotlin_compiler_and_real_browser_activation() {
    DocumentTitleBrowserFixture::run().await;
}
