use std::path::{Path, PathBuf};
use std::time::Duration;

struct PreferenceStringsBrowserFixture {
    root: PathBuf,
    binary: PathBuf,
    repo: PathBuf,
    source: std::ffi::OsString,
}

impl PreferenceStringsBrowserFixture {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/services/ChoiceService.java",
            include_str!(
                "../../tests/fixtures/tac-preference-strings/client/services/ChoiceService.java"
            ),
        ),
        (
            "client/pages/tac.html",
            include_str!("../../tests/fixtures/tac-preference-strings/client/pages/tac.html"),
        ),
        (
            "client/components/preference/choice/tac.html",
            include_str!(
                "../../tests/fixtures/tac-preference-strings/client/components/preference/choice/tac.html"
            ),
        ),
        (
            "client/manifest.json",
            include_str!("../../tests/fixtures/tac-preference-strings/client/manifest.json"),
        ),
    ];

    const EVENT_FILES: [(&'static str, &'static str); 3] = [
        (
            "client/models/PlatformChanged.java",
            include_str!(
                "../../tests/fixtures/tac-preference-strings/client/models/PlatformChanged.java"
            ),
        ),
        (
            "client/services/LateService.java",
            include_str!(
                "../../tests/fixtures/tac-preference-strings/client/services/LateService.java"
            ),
        ),
        (
            "client/components/preference/late/tac.html",
            include_str!(
                "../../tests/fixtures/tac-preference-strings/client/components/preference/late/tac.html"
            ),
        ),
    ];

    const PROBE_FILES: [(&'static str, &'static str); 2] = [
        (
            "client/services/SnippetProbeService.js",
            include_str!(
                "../../tests/fixtures/tac-preference-strings/client/services/SnippetProbeService.js"
            ),
        ),
        (
            "client/components/snippet/probe/tac.html",
            include_str!(
                "../../tests/fixtures/tac-preference-strings/client/components/snippet/probe/tac.html"
            ),
        ),
    ];

    const WEBSITE_FILES: [&'static str; 12] = [
        "client/services/PlatformChoiceService.java",
        "client/services/SnippetService.js",
        "client/services/ClipboardService.ts",
        "client/models/PlatformChanged.java",
        "client/models/PlatformChanged.ts",
        "client/models/SnippetFile.ts",
        "client/models/ShownFile.ts",
        "client/models/LanguageChoice.ts",
        "client/models/PlatformChoice.ts",
        "client/models/TaskScheduler.ts",
        "client/shared/scripts/platforms.js",
        "client/components/docs/snippet/tac.html",
    ];

    async fn run() {
        let fixture = Self::read();
        fixture.validate();
        fixture.stage();
        fixture.stage_website();
        fixture.build().await;
        fixture.browser().await;
    }

    fn read() -> Self {
        Self {
            root: PathBuf::from(Self::required("TAC_PREFERENCE_FIXTURE_OUT")),
            binary: PathBuf::from(Self::required("TAC_BIN")),
            repo: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            source: Self::required("TAC_PREFERENCE_SOURCE_COMMIT"),
        }
    }

    fn required(name: &str) -> std::ffi::OsString {
        std::env::var_os(name)
            .unwrap_or_else(|| panic!("{name} must explicitly identify this opt-in fixture"))
    }

    fn validate(&self) {
        assert!(self.root.is_absolute(), "fixture root must be absolute");
        assert!(self.binary.is_file(), "explicit compiler binary must exist");
        assert_eq!(
            crate::cache::root(),
            self.root.join("cache"),
            "TAC_CACHE_DIR must select the fixture's isolated cache"
        );
    }

    fn stage(&self) {
        for (relative, source) in Self::FILES
            .into_iter()
            .chain(Self::EVENT_FILES)
            .chain(Self::PROBE_FILES)
        {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap_or_else(|| panic!("fixture parent")))
                .unwrap_or_else(|error| panic!("fixture directory: {error}"));
            std::fs::write(path, source).unwrap_or_else(|error| panic!("fixture source: {error}"));
        }
    }

    fn stage_website(&self) {
        for relative in Self::WEBSITE_FILES {
            let target = self.root.join(relative);
            std::fs::create_dir_all(
                target
                    .parent()
                    .unwrap_or_else(|| panic!("website fixture parent")),
            )
            .unwrap_or_else(|error| panic!("website fixture directory: {error}"));
            std::fs::copy(self.repo.join("website").join(relative), target)
                .unwrap_or_else(|error| panic!("actual website fixture source: {error}"));
        }
    }

    async fn build(&self) {
        let mut command = tokio::process::Command::new(&self.binary);
        command
            .arg("bundle")
            .arg(&self.root)
            .args(["--out-dir", "dist", "--no-incremental"])
            .current_dir(&self.repo);
        Self::execute(&mut command, Duration::from_mins(6)).await;
    }

    async fn browser(&self) {
        let mut command = tokio::process::Command::new("node");
        command
            .arg(
                self.repo
                    .join("scripts/preference-strings-browser-gate.mjs"),
            )
            .env("TAC_BIN", &self.binary)
            .env("TAC_PREFERENCE_FIXTURE_OUT", &self.root)
            .env("TAC_PREFERENCE_SOURCE_COMMIT", &self.source)
            .current_dir(&self.repo);
        Self::execute(&mut command, Duration::from_mins(1)).await;
    }

    async fn execute(command: &mut tokio::process::Command, deadline: Duration) {
        let output = crate::external_command::run(command, deadline, 262_144)
            .await
            .unwrap_or_else(|error| panic!("supervised preference fixture: {error}"));
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
}

#[tokio::test]
#[ignore = "requires an explicit current compiler, isolated fixture/cache, TeaVM, JDK and installed Chromium"]
async fn java_preference_strings_use_the_ordinary_compiler_and_real_browser_activation() {
    PreferenceStringsBrowserFixture::run().await;
}
