#![allow(clippy::expect_used)]

use super::config::NativeApplication;
use super::native_menu_config::NativeMenuConfig;
use serde_json::{Value, json};
use tachyon_contracts::NativeTarget;

struct NativeMenuTests;

impl NativeMenuTests {
    fn project(menus: &Value) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("menu project");
        std::fs::create_dir_all(root.path().join("client")).expect("client directory");
        let manifest = json!({"name":"Menu Check","tac":{"application":{"menus":menus}}});
        std::fs::write(
            root.path().join("client/manifest.json"),
            manifest.to_string(),
        )
        .expect("real manifest");
        root
    }

    fn entry(id: &str, route: &str) -> Value {
        json!({"id":id,"label":"Open page","route":route})
    }

    fn views(root: &std::path::Path) {
        for (relative, source) in [
            ("client/pages/tac.html", "<home />"),
            ("client/pages/status/tac.html", "<status />"),
            (
                "client/components/home/tac.html",
                "<main>Browser app</main>",
            ),
            (
                "client/components/status/tac.html",
                "<main>Status page</main>",
            ),
        ] {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("view parent")).expect("view directory");
            std::fs::write(path, source).expect("real view");
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

    fn refusal(menus: &Value) -> String {
        let root = Self::project(menus);
        let failure =
            NativeApplication::discover(root.path()).expect_err("invalid menu declaration");
        assert!(
            failure.to_string().contains("client/manifest.json"),
            "{failure}"
        );
        failure.to_string()
    }
}

#[test]
fn native_menu_declarations_preserve_identity_and_emit_typed_local_actions() {
    let root = NativeMenuTests::project(
        &json!({"file":[{"id":"status","label":"Status \"local\"","route":"/status","shortcut":{"key":"1","modifiers":["command"]}}]}),
    );
    let application = NativeApplication::discover(root.path()).expect("typed declaration");
    assert_eq!(application.name, "Menu Check");
    application
        .menus
        .validate_for(NativeTarget::Macos, &NativeMenuTests::index())
        .expect("compiled page");
    let source = super::mac_os_menu_source::MacOsMenuSource::declarations(&application.menus);
    assert!(
        source.contains("MacOsMenuDestination(group: .file, route: \"/status\")"),
        "{source}"
    );
    assert!(source.contains("Status \\\"local\\\""), "{source}");
    assert!(source.contains("modifiers: [.command]"), "{source}");
    assert!(!source.contains("evaluateJavaScript"));
}

#[test]
fn native_menu_declarations_refuse_unknown_fields_and_unsafe_local_paths() {
    for menus in [
        json!({"service":[]}),
        json!({"file":[{"id":"x","label":"X","route":"/","selector":"terminate:"}]}),
        json!({"file":[{"id":"x","label":"X","route":"/","shortcut":{"key":"1","modifiers":["command"],"selector":"anything:"}}]}),
        json!({"file":[{"id":"x","label":"X","action":{"service":"MenuService","method":"run","script":"x"}}]}),
    ] {
        assert!(NativeMenuTests::refusal(&menus).contains("unknown field"));
    }
    NativeMenuTests::refusal(
        &json!({"file":[{"id":"x","label":"X","route":"/","action":"terminate:"}]}),
    );
    for route in [
        "https://example.com/",
        "//other/status",
        "/status?draft=1",
        "/status#part",
        "/a/../status",
        "/a//status",
        "/%73tatus",
        "/status\\hidden",
    ] {
        let error = NativeMenuTests::refusal(&json!({"file":[NativeMenuTests::entry("x",route)]}));
        assert!(error.contains("canonical local path"), "{route}: {error}");
    }
}

#[test]
fn native_menu_declarations_enforce_item_and_text_bounds() {
    for entry in [
        json!({"id":"","label":"X","route":"/"}),
        json!({"id":"a".repeat(65),"label":"X","route":"/"}),
        json!({"id":"x","label":"é".repeat(65),"route":"/"}),
        json!({"id":"x","label":"Bad\u{0000}label","route":"/"}),
    ] {
        NativeMenuTests::refusal(&json!({"file":[entry]}));
    }
    let entries = (0..65)
        .map(|i| NativeMenuTests::entry(&format!("page-{i}"), "/"))
        .collect::<Vec<_>>();
    assert!(NativeMenuTests::refusal(&json!({"view":entries})).contains("at most 64"));
    let duplicate = NativeMenuTests::entry("same", "/");
    assert!(
        NativeMenuTests::refusal(&json!({"file":[duplicate.clone()],"help":[duplicate]}))
            .contains("ids must be unique")
    );
}

#[test]
fn native_menu_declarations_refuse_typing_interception_reserved_and_duplicate_shortcuts() {
    for shortcut in [
        json!({"key":"1","modifiers":[]}),
        json!({"key":"1","modifiers":["control"]}),
        json!({"key":"1","modifiers":["command","command"]}),
        json!({"key":"1","modifiers":["command","unknown"]}),
        json!({"key":"Q","modifiers":["command"]}),
        json!({"key":"q","modifiers":["command"]}),
        json!({"key":"h","modifiers":["option","command"]}),
        json!({"key":"f","modifiers":["control","command"]}),
    ] {
        NativeMenuTests::refusal(
            &json!({"file":[{"id":"x","label":"X","route":"/","shortcut":shortcut}]}),
        );
    }
    let one = json!({"id":"one","label":"One","route":"/","shortcut":{"key":"1","modifiers":["command","shift"]}});
    let two = json!({"id":"two","label":"Two","route":"/status","shortcut":{"key":"1","modifiers":["shift","command"]}});
    assert!(
        NativeMenuTests::refusal(&json!({"file":[one],"help":[two]}))
            .contains("shortcuts must be unique")
    );
}

#[test]
fn native_menu_declarations_require_exact_compiled_static_documents_and_supported_target() {
    for route in [
        "/missing",
        "/status/",
        "/status/index.html",
        "/topic/value",
        "/topic/_slug",
    ] {
        let root = NativeMenuTests::project(&json!({"file":[NativeMenuTests::entry("x",route)]}));
        let application = NativeApplication::discover(root.path()).expect("canonical path syntax");
        let failure = application
            .menus
            .validate_for(NativeTarget::Macos, &NativeMenuTests::index())
            .expect_err("not a compiled static page key");
        assert!(
            failure
                .to_string()
                .contains("exactly identify a compiled static page"),
            "{failure}"
        );
    }
    let root = NativeMenuTests::project(&json!({"file":[NativeMenuTests::entry("x","/")]}));
    let application = NativeApplication::discover(root.path()).expect("local page");
    for target in [
        NativeTarget::Macos,
        NativeTarget::Linux,
        NativeTarget::Windows,
    ] {
        application
            .menus
            .validate_for(target, &NativeMenuTests::index())
            .expect("desktop hosts have a menu bar");
    }
    for target in [NativeTarget::Ios, NativeTarget::Android] {
        assert!(
            application
                .menus
                .validate_for(target, &NativeMenuTests::index())
                .expect_err("unsupported target")
                .to_string()
                .contains("only on macOS, Linux and Windows")
        );
        NativeMenuConfig::default()
            .validate_for(target, &NativeMenuTests::index())
            .expect("ordinary target unchanged");
    }
}

#[test]
fn native_menu_declarations_do_not_apply_native_route_constraints_to_browser_builds() {
    let root =
        NativeMenuTests::project(&json!({"help":[NativeMenuTests::entry("help","/native-only")]}));
    NativeMenuTests::views(root.path());
    let result = crate::WebCompiler::build(root.path(), &crate::BuildOptions::default())
        .expect("ordinary browser ignores native menu routes");
    assert!(result.output_directory().join("index.html").is_file());
}

#[tokio::test]
async fn native_menu_declarations_stage_real_compiled_routes_without_packaging() {
    let root =
        NativeMenuTests::project(&json!({"file":[NativeMenuTests::entry("status","/status")]}));
    NativeMenuTests::views(root.path());
    let options = super::compiler::NativeBuildOptions {
        output_directory: root.path().join("dist"),
        target: NativeTarget::Macos,
        package: false,
        ..Default::default()
    };
    let result = super::compiler::NativeCompiler::build_async(root.path(), &options)
        .await
        .expect("ordinary native compiler stages declared compiled route");
    let host = std::fs::read_to_string(result.output_directory().join("project/TachyonHost.swift"))
        .expect("real generated host");
    assert!(
        host.contains("MacOsMenuDestination(group: .file, route: \"/status\")"),
        "{host}"
    );
    assert!(host.contains("MacOsMenuNavigation(view, window, tachyonRouteDocuments"));
    assert!(host.contains("view?.load(URLRequest(url: url))"));
    assert!(!host.contains("__NATIVE_MENU_ROUTES__"));
}
