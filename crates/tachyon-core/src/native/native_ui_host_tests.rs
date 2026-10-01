#![allow(clippy::expect_used)]

use super::{
    native_ui_host_sources::NativeUiHostSources, native_ui_styles_sources::NativeUiStylesSources,
    native_ui_styles_tests::NativeUiStylesFixture,
};
use std::path::{Path, PathBuf};

struct NativeUiHostFixture {
    fixture: NativeUiStylesFixture,
}

impl NativeUiHostFixture {
    fn new() -> Self {
        Self {
            fixture: NativeUiStylesFixture::new(),
        }
    }

    fn index() -> super::routes::NativeRouteIndex {
        super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: "/".to_owned(),
            entry_document: "index.html".to_owned(),
            routes: [
                ("/", "index.html"),
                ("/status", "status/index.html"),
                ("/topic/_slug", "topic/_slug/index.html"),
            ]
            .into_iter()
            .map(|(route, document)| super::routes::NativeRouteIndexEntry {
                route: route.to_owned(),
                document: document.to_owned(),
            })
            .collect(),
        }
    }

    fn original(&self, companion: &Path) -> String {
        self.fixture.write("client/manifest.json", br#"{"name":"Native Live Host","tac":{"application":{"id":"org.example.live","version":"1"}}}"#);
        let application = super::config::NativeApplication::discover(self.fixture.root())
            .expect("normal generated host configuration");
        super::macos::swift_source(
            &application,
            super::macos::companion_call(Some(companion), None),
            &Self::index(),
        )
    }

    fn host(&self, companion: &Path) -> PathBuf {
        let original = self.original(companion);
        self.fixture
            .write("GeneratedHost.swift", original.as_bytes());
        let injected =
            NativeUiHostSources::host(&original).expect("bounded generated fixture seam");
        self.fixture.write("LiveHost.swift", injected.as_bytes())
    }

    async fn compile(&self) -> PathBuf {
        let companion = self.fixture.companion();
        let mut arguments = vec![
            "swiftc".to_owned(),
            "-parse-as-library".to_owned(),
            "-framework".to_owned(),
            "AppKit".to_owned(),
            "-framework".to_owned(),
            "WebKit".to_owned(),
        ];
        for (name, source) in NativeUiStylesSources::sources() {
            arguments.push(
                self.fixture
                    .write(&name, source.as_bytes())
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        let script = NativeUiStylesSources::swift_script();
        let caller = NativeUiHostSources::caller().expect("actual host bridge caller");
        let sources = [
            ("StyleScript.swift", script.as_str()),
            (
                "StyleSession.swift",
                include_str!("live/NativeUiStyleSession.swift"),
            ),
            (
                "StyleHost.swift",
                include_str!("live/NativeUiStyleHost.swift"),
            ),
            (
                "StyleHostScheme.swift",
                include_str!("live/NativeUiStyleHostScheme.swift"),
            ),
            ("StyleCheck.swift", caller.as_str()),
        ];
        arguments.extend(sources.into_iter().map(|(name, text)| {
            self.fixture
                .write(name, text.as_bytes())
                .to_string_lossy()
                .into_owned()
        }));
        arguments.push(self.host(&companion).to_string_lossy().into_owned());
        arguments.push(companion.to_string_lossy().into_owned());
        let executable = self.fixture.root().join("native-live-host-check");
        arguments.extend(["-o".to_owned(), executable.to_string_lossy().into_owned()]);
        super::host::run_tool(
            "/usr/bin/xcrun",
            &arguments.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .await
        .expect("actual generated host Swift compile");
        executable
    }

    fn generation(&self, folder: &str, style: &str, fingerprint: &str) -> [String; 3] {
        let document = NativeUiStylesFixture::DOCUMENT.replace(
            "<main id=\"sample\">",
            "<main id=\"sample\" data-route=\"root\">",
        );
        self.fixture
            .write(&format!("{folder}/index.html"), document.as_bytes());
        self.fixture.write(
            &format!("{folder}/status/index.html"),
            document
                .replace("data-route=\"root\"", "data-route=\"static\"")
                .as_bytes(),
        );
        self.fixture.write(
            &format!("{folder}/topic/_slug/index.html"),
            document
                .replace("data-route=\"root\"", "data-route=\"dynamic\"")
                .as_bytes(),
        );
        self.fixture.write(
            &format!("{folder}/topic/_slug/detail.txt"),
            b"captured nested route asset",
        );
        self.fixture
            .write(&format!("{folder}/styles/theme.css"), style.as_bytes());
        self.fixture.capture_generation(folder, fingerprint)
    }

    fn fingerprint(executable: &Path) -> String {
        let bytes =
            std::fs::read(executable).expect("actual independently compiled fixture binary");
        super::native_ui_descriptor::NativeUiDescriptor::sha256(&bytes)
    }
}

#[test]
fn native_ui_host_fixture_preserves_default_generator_and_requires_exact_owned_seams() {
    let fixture = NativeUiHostFixture::new();
    let companion = fixture.fixture.companion();
    let original = fixture.original(&companion);
    assert!(!original.contains("NativeUiStyleHost"));
    let injected = NativeUiHostSources::host(&original).expect("opt-in fixture");
    assert!(injected.contains("liveStyleHost.configure(configuration, tachyonDocument)"));
    assert!(injected.contains("liveStyleHost.invalidate()"));
    assert!(injected.contains("fixtureFrame.configure(configuration)"));
    assert_eq!(
        injected
            .matches("guard !fixtureLaunched else { return }")
            .count(),
        1
    );
    assert!(injected.contains("addScriptMessageHandler("));
    assert!(injected.contains("tachyonDisposeCompanions()"));
    assert!(!injected.contains("let delegate = AppDelegate()"));
    assert!(
        NativeUiHostSources::host(&original.replace(
            "private final class AppDelegate",
            "private final class RenamedDelegate"
        ))
        .is_err()
    );
    assert_eq!(
        fixture.original(&companion),
        original,
        "ordinary generator remains byte-identical"
    );
}

#[tokio::test]
async fn native_ui_host_fixture_compiles_generated_appdelegate_and_quiet_capture_controls() {
    let fixture = NativeUiHostFixture::new();
    let executable = fixture.compile().await;
    let receipt = NativeUiStylesFixture::execute(&executable, &["css"]).await;
    assert!(
        receipt.contains("native-ui-styles-css-profile-passed"),
        "{receipt}"
    );
    let fingerprint = NativeUiHostFixture::fingerprint(&executable);
    let installed = fixture.generation("installed", "main { color: black; }", &fingerprint);
    let candidate = fixture.generation("candidate", "main { color: blue; }", &fingerprint);
    let arguments = std::iter::once("store")
        .chain(installed.iter().chain(candidate.iter()).map(String::as_str))
        .collect::<Vec<_>>();
    let controls = NativeUiStylesFixture::execute(&executable, &arguments).await;
    assert!(
        controls.contains("native-ui-host-route-controls-passed"),
        "{controls}"
    );
}

#[tokio::test]
#[ignore = "Requires actual generated AppDelegate AppKit/WebKit window and focus; run explicitly in the coordinator SDK lane"]
async fn native_ui_host_fixture_keeps_real_bridge_reply_and_state_across_styles_and_disposes_on_navigation()
 {
    let fixture = NativeUiHostFixture::new();
    let executable = fixture.compile().await;
    let fingerprint = NativeUiHostFixture::fingerprint(&executable);
    let installed = fixture.generation("installed", "main { color: black; }", &fingerprint);
    let candidate = fixture.generation("candidate", "main { color: blue; }", &fingerprint);
    let arguments = installed
        .iter()
        .chain(candidate.iter())
        .map(String::as_str)
        .collect::<Vec<_>>();
    let receipt = NativeUiStylesFixture::execute(&executable, &arguments).await;
    assert!(
        receipt.contains("native-ui-styles-webkit-passed pid="),
        "{receipt}"
    );
}
