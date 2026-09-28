#![allow(clippy::expect_used)]

use super::registry::{NativeCompanionInput, source};
use crate::annotation_layout::NativeWebApis;
use crate::project::NativeCompanion;
use std::path::Path;

struct Services;

impl Services {
    const RUST_CLIPBOARD: &str = "#[Service]\n#[derive(Default)]\nstruct PlaceService {\n    clipboard: Clipboard,\n}\n\nimpl PlaceService {\n    fn copy(&self) {\n        let _ = self.clipboard.write_text(\"place\");\n    }\n}\n";

    const CSHARP_LISTENER: &str = "[Service]\npublic class PlaceService\n{\n    public bool Online = false;\n\n    [OnEvent]\n    public void Hear(ConnectivityChanged changed)\n    {\n        Online = changed.Online;\n    }\n}\n";

    fn built(
        language: NativeCompanion,
        extension: &str,
        authored: &str,
        web_assembly: bool,
    ) -> Result<(), String> {
        let root = tempfile::tempdir().expect("project");
        let path = root
            .path()
            .join(format!("client/services/PlaceService.{extension}"));
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, authored).expect("service");
        let input = NativeCompanionInput {
            snapshot_root: root.path().to_path_buf(),
            language,
            source: path,
            service: String::from("place"),
            class: String::from("PlaceService"),
            web_assembly,
        };
        source(&[input], language)
            .map(|_| ())
            .map_err(|failure| failure.to_string())
    }
}

#[test]
fn a_linked_rust_service_cannot_use_a_browser_api() {
    let error = Services::built(NativeCompanion::Rust, "rs", Services::RUST_CLIPBOARD, false)
        .expect_err("refused");
    assert!(error.contains("TY1317"), "{error}");
    assert!(
        error.contains(
            "Rust services that are linked into the app cannot use `Clipboard`: it belongs to the page."
        ),
        "{error}"
    );
}

#[test]
fn a_rust_service_in_the_page_keeps_its_browser_api() {
    Services::built(NativeCompanion::Rust, "rs", Services::RUST_CLIPBOARD, true)
        .expect("a WebAssembly service reaches the page");
}

#[test]
fn a_linked_csharp_service_cannot_listen_for_a_page_event() {
    let error = Services::built(
        NativeCompanion::CSharp,
        "cs",
        Services::CSHARP_LISTENER,
        false,
    )
    .expect_err("refused");
    assert!(
        error.contains("C# services that are linked into the app cannot use `ConnectivityChanged`"),
        "{error}"
    );
}

#[test]
fn swift_and_kotlin_keep_their_own_clock_but_not_page_events() {
    let kotlin = Path::new("client/services/PlaceService.kt");
    NativeWebApis::refuse(kotlin, "val now = Clock.System.now()\n").expect("Kotlin's own Clock");
    let swift = Path::new("client/services/PlaceService.swift");
    NativeWebApis::refuse(swift, "let clock = ContinuousClock()\n// PositionFound\n")
        .expect("comments do not count");
    let error = NativeWebApis::refuse(swift, "func found(_ found: PositionFound) {}\n")
        .expect_err("refused")
        .to_string();
    assert!(
        error.contains("Swift services that are linked into the app cannot use `PositionFound`"),
        "{error}"
    );
}
