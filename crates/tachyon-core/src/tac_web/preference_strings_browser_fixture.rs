use super::ordinary_browser_fixture::OrdinaryBrowserFixture;

struct PreferenceStringsBrowserFixture {
    environment: OrdinaryBrowserFixture,
}

impl PreferenceStringsBrowserFixture {
    const ENVIRONMENT: [&'static str; 3] = [
        "TAC_PREFERENCE_FIXTURE_OUT",
        "TAC_BIN",
        "TAC_PREFERENCE_SOURCE_COMMIT",
    ];
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
        "client/shared/utils/PlatformUtils.js",
        "client/components/docs/snippet/tac.html",
    ];

    async fn run() {
        let fixture = Self::read();
        fixture.environment.validate();
        fixture.stage();
        fixture.stage_website();
        fixture.environment.build().await;
        fixture.browser().await;
    }

    fn read() -> Self {
        Self {
            environment: OrdinaryBrowserFixture::read(Self::ENVIRONMENT),
        }
    }

    fn stage(&self) {
        self.environment.stage(
            Self::FILES
                .into_iter()
                .chain(Self::EVENT_FILES)
                .chain(Self::PROBE_FILES),
        );
    }

    fn stage_website(&self) {
        for relative in Self::WEBSITE_FILES {
            let target = self.environment.root.join(relative);
            std::fs::create_dir_all(
                target
                    .parent()
                    .unwrap_or_else(|| panic!("website fixture parent")),
            )
            .unwrap_or_else(|error| panic!("website fixture directory: {error}"));
            std::fs::copy(self.environment.repo.join("website").join(relative), target)
                .unwrap_or_else(|error| panic!("actual website fixture source: {error}"));
        }
    }

    async fn browser(&self) {
        self.environment
            .browser(
                Self::ENVIRONMENT,
                "scripts/preference-strings-browser-gate.mjs",
            )
            .await;
    }
}

#[tokio::test]
#[ignore = "requires an explicit current compiler, isolated fixture/cache, TeaVM, JDK and installed Chromium"]
async fn java_preference_strings_use_the_ordinary_compiler_and_real_browser_activation() {
    PreferenceStringsBrowserFixture::run().await;
}
