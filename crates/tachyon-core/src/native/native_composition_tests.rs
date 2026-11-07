#![allow(clippy::expect_used)]

use crate::native::native_page_graph_tests::NativePageGraphCheck;
use crate::{ProjectDiscovery, native::registry::NativeCompanionInput};
use std::path::Path;

struct NativeCompositionCheck;

impl NativeCompositionCheck {
    const ROOT: &str = "#[Service]\nstruct SnapshotReaderService { reader: EndpointService }\nimpl Default for SnapshotReaderService { fn default() -> Self { Self { reader: EndpointService::new(String::from(\"authored endpoint\")) } } }\nimpl SnapshotReaderService { pub fn read(&self) -> i64 { self.reader.read() } }\n";
    const ENDPOINT: &str = "#[Service]\nstruct EndpointService { endpoint: String }\nimpl EndpointService { fn new(endpoint: String) -> Self { Self { endpoint } } fn read(&self) -> i64 { self.endpoint.len() as i64 } }\n";
    const CALLER: &str = r###"
fn main() {
    let owner = "00000000-0000-4000-8000-000000000001";
    let init = tac_handle(&format!(r##"{{"route":"/","service":"snapshotReader","owner":"{owner}","op":"init"}}"##));
    assert!(!init.contains("error"), "{init}");
    let value = tac_handle(&format!(r##"{{"route":"/","service":"snapshotReader","owner":"{owner}","op":"call","name":"read","args":[]}}"##));
    assert_eq!(value, "EXPECTED");
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
}
"###;

    fn write(root: &Path, path: &str, source: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, source).expect("fixture");
    }

    fn project(root: &Path, view: &str) {
        Self::write(root, "client/pages/tac.html", "<repro-panel />");
        Self::write(root, "client/components/repro-panel/tac.html", view);
        Self::write(root, "client/services/SnapshotReaderService.rs", Self::ROOT);
        Self::write(
            root,
            "client/services/native/EndpointService.rs",
            Self::ENDPOINT,
        );
    }

    fn trait_project(root: &Path, view: &str) {
        Self::project(root, view);
        Self::write(
            root,
            "client/shared/native/ObservationClock.rs",
            "pub trait ObservationClock { fn now(&self) -> i64; }\n",
        );
        Self::write(
            root,
            "client/services/native/ClockReaderService.rs",
            "use crate::ObservationClock as ClockBoundary;\n#[Service]\nstruct ClockReaderService { clock: std::sync::Arc<dyn ClockBoundary> }\nimpl ClockReaderService { fn new(clock: std::sync::Arc<dyn ClockBoundary>) -> Self { Self { clock } } fn read(&self) -> i64 { self.clock.now() } }\n",
        );
        Self::write(
            root,
            "client/services/native/SystemClockService.rs",
            "use crate::ObservationClock;\n#[Service]\n#[derive(Default)]\nstruct SystemClockService;\nimpl crate::ObservationClock for SystemClockService { fn now(&self) -> i64 { 1 } }\n",
        );
        Self::write(
            root,
            "client/services/SnapshotReaderService.rs",
            "use std::io::Result;\n#[Service]\nstruct SnapshotReaderService { reader: ClockReaderService }\nimpl Default for SnapshotReaderService { fn default() -> Self { Self { reader: ClockReaderService::new(std::sync::Arc::new(SystemClockService::default())) } } }\nimpl SnapshotReaderService { pub fn read(&self) -> i64 { self.reader.read() } }\n",
        );
    }

    fn unread_root(root: &Path) {
        let path = root.join("client/services/SnapshotReaderService.rs");
        let source = std::fs::read_to_string(&path).expect("root service");
        std::fs::write(path, source.replace("pub fn read", "fn read")).expect("private root");
    }

    async fn inputs(root: &Path) -> (crate::Project, tempfile::TempDir, Vec<NativeCompanionInput>) {
        let project = ProjectDiscovery::discover(root).expect("project");
        let output_root = root.join("native-output");
        let options = super::NativeBuildOptions::default();
        let previous = output_root.join("macos");
        let (web, _built, inputs) =
            super::build_web_bundle(&project, &options, &output_root, &previous)
                .await
                .expect("ordinary native collection");
        (project, web, inputs)
    }

    async fn execute(root: &Path, expected: &str) {
        let (_project, web, inputs) = Self::inputs(root).await;
        assert_eq!(
            inputs
                .iter()
                .map(|input| input.service.as_str())
                .collect::<Vec<_>>(),
            ["snapshotReader"]
        );
        let source =
            crate::native::rust::stage(&inputs, web.path(), "org.tachyon.explicit-composition")
                .expect("explicit staging")
                .expect("Rust source");
        let emitted = std::fs::read_to_string(&source).expect("source");
        assert!(emitted.contains("struct EndpointService"));
        assert!(!emitted.contains("fn tac_definition_EndpointService"));
        NativePageGraphCheck::execute(&source, &Self::CALLER.replace("EXPECTED", expected)).await;
    }
}

#[tokio::test]
async fn native_cli_collection_preserves_explicit_scalar_composition() {
    let root = tempfile::tempdir().expect("fixture");
    NativeCompositionCheck::project(root.path(), "<p>{snapshotReader.read()}</p>");
    NativeCompositionCheck::execute(root.path(), "{\\\"value\\\":17}").await;
}

#[tokio::test]
async fn native_cli_collection_still_refuses_view_bound_automatic_scalar_injection() {
    let root = tempfile::tempdir().expect("fixture");
    NativeCompositionCheck::project(root.path(), "<p>{endpoint.read()}</p>");
    NativeCompositionCheck::unread_root(root.path());
    let (_project, web, inputs) = NativeCompositionCheck::inputs(root.path()).await;
    assert_eq!(
        inputs
            .iter()
            .map(|input| input.service.as_str())
            .collect::<Vec<_>>(),
        ["endpoint"]
    );
    let failure = crate::native::rust::stage(&inputs, web.path(), "org.tachyon.required-scalar")
        .expect_err("no inferred scalar provider");
    assert!(
        failure.to_string().contains("provider 'String'"),
        "{failure}"
    );
}

#[tokio::test]
async fn native_cli_collection_tracks_nested_conditionals_and_exported_elements() {
    let root = tempfile::tempdir().expect("fixture");
    NativeCompositionCheck::project(
        root.path(),
        "<section><logic :if=\"true\"><nested-reader /></logic></section>",
    );
    NativeCompositionCheck::write(
        root.path(),
        "client/components/nested-reader/tac.html",
        "<p>{snapshotReader.read()}</p>",
    );
    NativeCompositionCheck::write(
        root.path(),
        "client/components/exported-reader/tac.html",
        "<p>{endpoint.read()}</p>",
    );
    NativeCompositionCheck::write(
        root.path(),
        "client/manifest.json",
        r#"{"tac":{"elements":["exported-reader"]}}"#,
    );
    let (_project, _web, inputs) = NativeCompositionCheck::inputs(root.path()).await;
    assert_eq!(
        inputs
            .iter()
            .map(|input| input.service.as_str())
            .collect::<Vec<_>>(),
        ["endpoint", "snapshotReader"]
    );
}

#[tokio::test]
async fn native_cli_collection_preserves_explicit_trait_composition_and_local_paths() {
    let root = tempfile::tempdir().expect("fixture");
    NativeCompositionCheck::trait_project(root.path(), "<p>{snapshotReader.read()}</p>");
    NativeCompositionCheck::execute(root.path(), "{\\\"value\\\":1}").await;
}

#[tokio::test]
async fn native_cli_collection_preserves_rust_utility_boundary_imports() {
    let utilities = [
        "pub struct TitleUtils;\nimpl TitleUtils { pub fn text() -> String { String::from(\"Ready\") } }\n",
        "pub struct TitleUtils;\nimpl TitleUtils { const TITLE: &'static str = \"Ready\"; pub fn text() -> String { Self::TITLE.to_owned() } }\n",
    ];
    for utility in utilities {
        let root = tempfile::tempdir().expect("fixture");
        NativeCompositionCheck::project(root.path(), "<p>{snapshotReader.read()}</p>");
        NativeCompositionCheck::write(root.path(), "client/shared/utils/TitleUtils.rs", utility);
        NativeCompositionCheck::write(
            root.path(),
            "client/services/SnapshotReaderService.rs",
            "use crate::utils::{TitleUtils, TitleUtils as Label};\n#[Service]\n#[derive(Default)]\nstruct SnapshotReaderService;\nimpl SnapshotReaderService { pub fn read(&self) -> String { mod nested { pub fn value() -> String { crate::utils::TitleUtils::text() } } format!(\"{}:{}:{}\", TitleUtils::text(), Label::text(), nested::value()) } }\n",
        );
        NativeCompositionCheck::execute(root.path(), "{\\\"value\\\":\\\"Ready:Ready:Ready\\\"}")
            .await;
    }
}

#[tokio::test]
async fn native_cli_collection_still_refuses_view_bound_automatic_trait_injection() {
    let root = tempfile::tempdir().expect("fixture");
    NativeCompositionCheck::trait_project(root.path(), "<p>{clockReader.read()}</p>");
    NativeCompositionCheck::unread_root(root.path());
    let (_project, web, inputs) = NativeCompositionCheck::inputs(root.path()).await;
    let failure = crate::native::rust::stage(&inputs, web.path(), "org.tachyon.required-trait")
        .expect_err("no automatic trait binding");
    assert!(
        failure.to_string().contains("dyn ClockBoundary"),
        "{failure}"
    );
}

#[tokio::test]
async fn tac_utility_canonical_nested_rust_module_executes_in_native_composition() {
    let root = tempfile::tempdir().expect("fixture");
    NativeCompositionCheck::project(root.path(), "<p>{snapshotReader.read()}</p>");
    NativeCompositionCheck::write(
        root.path(),
        "client/shared/utils/text/TitleUtils.rs",
        "pub struct TitleUtils;\nimpl TitleUtils { pub fn text() -> String { String::from(\"Ready\") } }\n",
    );
    NativeCompositionCheck::write(
        root.path(),
        "client/services/SnapshotReaderService.rs",
        "use crate::utils::{TitleUtils, TitleUtils as Label};\n#[Service]\n#[derive(Default)]\nstruct SnapshotReaderService;\nimpl SnapshotReaderService { pub fn read(&self) -> String { mod nested { pub fn value() -> String { crate::utils::TitleUtils::text() } } format!(\"{}:{}:{}\", TitleUtils::text(), Label::text(), nested::value()) } }\n",
    );
    NativeCompositionCheck::execute(root.path(), "{\\\"value\\\":\\\"Ready:Ready:Ready\\\"}").await;
}

#[tokio::test]
async fn native_cli_collection_follows_script_instance_dependencies_transitively() {
    let root = tempfile::tempdir().expect("fixture");
    NativeCompositionCheck::project(root.path(), "<p>{observatory.read()}</p>");
    NativeCompositionCheck::write(
        root.path(),
        "client/services/ObservatoryService.ts",
        "import SnapshotService from './SnapshotService';\n@Service()\nexport default class ObservatoryService {\n @Needs()\n private snapshot!: SnapshotService;\n read() { return this.snapshot.read(); }\n}\n",
    );
    NativeCompositionCheck::write(
        root.path(),
        "client/services/SnapshotService.ts",
        "import SnapshotReaderService from './SnapshotReaderService';\n@Service()\nexport default class SnapshotService {\n @Needs()\n private snapshotReader!: SnapshotReaderService;\n read() { return this.snapshotReader.read(); }\n}\n",
    );
    NativeCompositionCheck::write(
        root.path(),
        "client/services/SnapshotReaderService.ts",
        "@Service()\nexport default class SnapshotReaderService { read() { return 0; } }\n",
    );
    NativeCompositionCheck::execute(root.path(), "{\\\"value\\\":17}").await;
}

#[tokio::test]
async fn native_cli_collection_stops_script_dependencies_at_selected_native_companions() {
    let root = tempfile::tempdir().expect("fixture");
    NativeCompositionCheck::project(root.path(), "<p>{snapshotReader.read()}</p>");
    NativeCompositionCheck::write(
        root.path(),
        "client/services/SnapshotReaderService.ts",
        "import EndpointService from './native/EndpointService';\n@Service()\nexport default class SnapshotReaderService {\n @Needs()\n private endpoint!: EndpointService;\n read() { return this.endpoint.read(); }\n}\n",
    );
    NativeCompositionCheck::write(
        root.path(),
        "client/services/native/EndpointService.ts",
        "@Service()\nexport default class EndpointService { read() { return 0; } }\n",
    );
    NativeCompositionCheck::execute(root.path(), "{\\\"value\\\":17}").await;
}

#[tokio::test]
async fn native_cli_collection_keeps_unmarked_browser_instance_name_bindings() {
    let root = tempfile::tempdir().expect("fixture");
    NativeCompositionCheck::project(root.path(), "<p>{observatory.read()}</p>");
    NativeCompositionCheck::write(
        root.path(),
        "client/services/ObservatoryService.ts",
        "import SnapshotReaderService from './SnapshotReaderService';\n@Service()\nexport default class ObservatoryService {\n private snapshotReader!: SnapshotReaderService;\n read() { return this.snapshotReader.read(); }\n}\n",
    );
    NativeCompositionCheck::write(
        root.path(),
        "client/services/SnapshotReaderService.ts",
        "@Service()\nexport default class SnapshotReaderService { read() { return 0; } }\n",
    );
    NativeCompositionCheck::execute(root.path(), "{\\\"value\\\":17}").await;
}
