#![allow(clippy::expect_used)]

use super::{
    NativeUiContract, NativeUiGeneration, native_ui_styles_sources::NativeUiStylesSources,
};
use cap_std::{ambient_authority, fs::Dir};
use std::path::{Path, PathBuf};

struct NativeUiStylesFixture {
    root: tempfile::TempDir,
}

impl NativeUiStylesFixture {
    const SERVICE: &str = "@Service\nfinal class LiveCounterService {\n    static var starts: Int = 0\n    static var stops: Int = 0\n    var count: Int = 0\n    @OnStart\n    func start() { Self.starts += 1 }\n    @OnStop\n    func stop() { Self.stops += 1 }\n    func increment() -> Int { count += 1; return count }\n    func read() -> Int { count }\n    func identity() -> Int { ObjectIdentifier(self).hashValue }\n}\n";
    const DOCUMENT: &str = r#"<!DOCTYPE html><html><head><link rel="stylesheet" href="/styles/theme.css"></head><body><input id="field" value="initial"><button id="click">Click</button><main id="sample">Native live style fixture</main><script>window.marker=Math.random().toString(36);window.events=0;document.querySelector('#click').addEventListener('click',()=>{window.events+=1})</script></body></html>"#;

    fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("stylesheet fixture"),
        }
    }

    fn write(&self, path: &str, bytes: &[u8]) -> PathBuf {
        let target = self.root.path().join(path);
        std::fs::create_dir_all(target.parent().expect("parent")).expect("directory");
        std::fs::write(&target, bytes).expect("fixture source");
        target
    }

    fn generation(&self, folder: &str, style: &str) -> [String; 3] {
        self.write(&format!("{folder}/index.html"), Self::DOCUMENT.as_bytes());
        self.write(&format!("{folder}/styles/theme.css"), style.as_bytes());
        let root = self.root.path().join(folder);
        let dir = Dir::open_ambient_dir(&root, ambient_authority())
            .expect("native test-owned trust authority");
        let contract = NativeUiContract::macos("org.example.live".to_owned(), "a".repeat(64))
            .expect("contract");
        let generation =
            NativeUiGeneration::capture(&dir, contract).expect("trusted source capture");
        let descriptor = self.write(
            &format!("{folder}.json"),
            &generation.descriptor_bytes().expect("descriptor"),
        );
        [
            root.to_string_lossy().into_owned(),
            descriptor.to_string_lossy().into_owned(),
            generation
                .descriptor_sha256()
                .expect("independent native anchor"),
        ]
    }

    fn companion(&self) -> PathBuf {
        let source = self.write(
            "client/services/LiveCounterService.swift",
            Self::SERVICE.as_bytes(),
        );
        let input = super::registry::NativeCompanionInput {
            snapshot_root: self.root.path().to_owned(),
            language: crate::project::NativeCompanion::Swift,
            source,
            service: "liveCounter".to_owned(),
            class: "LiveCounterService".to_owned(),
            web_assembly: false,
        };
        super::macos::stage_swift_companion(&[input], self.root.path())
            .expect("actual staged native Service")
            .expect("companion")
    }

    async fn compile(&self) -> PathBuf {
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
                self.write(&name, source.as_bytes())
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        let script = NativeUiStylesSources::swift_script();
        let extra = [
            ("StyleScript.swift", script.as_str()),
            (
                "StyleSession.swift",
                include_str!("live/NativeUiStyleSession.swift"),
            ),
            (
                "StyleScene.swift",
                include_str!("live/NativeUiStyleScene.fixture.swift"),
            ),
            (
                "StyleCheck.swift",
                include_str!("live/NativeUiStyleCheck.fixture.swift"),
            ),
            (
                "AppleJson.swift",
                super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST,
            ),
        ];
        arguments.extend(extra.into_iter().map(|(name, text)| {
            self.write(name, text.as_bytes())
                .to_string_lossy()
                .into_owned()
        }));
        arguments.push(self.companion().to_string_lossy().into_owned());
        let executable = self.root.path().join("style-check");
        arguments.extend(["-o".to_owned(), executable.to_string_lossy().into_owned()]);
        super::host::run_tool(
            "/usr/bin/xcrun",
            &arguments.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .await
        .expect("Swift/WebKit compile");
        executable
    }

    async fn execute(executable: &Path, arguments: &[&str]) -> String {
        super::host::run_tool(&executable.to_string_lossy(), arguments)
            .await
            .expect("supervised actual stylesheet fixture")
    }
}

#[tokio::test]
async fn native_ui_styles_compiles_and_rejects_unsupported_css_without_opening_a_window() {
    let fixture = NativeUiStylesFixture::new();
    let executable = fixture.compile().await;
    let result = NativeUiStylesFixture::execute(&executable, &["css"]).await;
    assert!(
        result.contains("native-ui-styles-css-profile-passed"),
        "{result}"
    );
}

#[tokio::test]
#[ignore = "Requires actual AppKit/WKWebView window and focus; run explicitly in the coordinator SDK lane"]
async fn native_ui_styles_keeps_real_window_document_native_service_state_and_rolls_back() {
    let fixture = NativeUiStylesFixture::new();
    let installed = fixture.generation("installed", "main { color: black; }");
    let candidate = fixture.generation("candidate", "main { color: blue; }");
    let executable = fixture.compile().await;
    let arguments = installed
        .iter()
        .chain(candidate.iter())
        .map(String::as_str)
        .collect::<Vec<_>>();
    let result = NativeUiStylesFixture::execute(&executable, &arguments).await;
    assert!(
        result.contains("native-ui-styles-webkit-passed pid="),
        "{result}"
    );
}
