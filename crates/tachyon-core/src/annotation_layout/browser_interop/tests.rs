#![allow(clippy::expect_used)]

use super::BrowserInteropCheck;
use std::path::Path;

fn refused(relative: &str, source: &str) -> Vec<String> {
    BrowserInteropCheck::problems(Path::new(relative), source)
        .into_iter()
        .map(|problem| format!("{} {}", problem.code, problem.message))
        .collect()
}

fn found_text(relative: &str, source: &str) -> Vec<String> {
    BrowserInteropCheck::problems(Path::new(relative), source)
        .into_iter()
        .filter_map(|problem| problem.help)
        .collect()
}

#[test]
fn a_go_service_may_not_import_syscall_js() {
    let source =
        "import (\n\t\"fmt\"\n\t\"syscall/js\"\n)\n\n// @Service\ntype PlaceService struct{}\n";
    let problems = refused("client/services/PlaceService.go", source);
    assert_eq!(
        problems,
        [
            "TY1317 Go services cannot call browser APIs. Put this code in a JS or TS service and send the result to this service with an event."
        ]
    );
    let aliased = "import browser \"syscall/js\"\n";
    assert_eq!(refused("client/services/PlaceService.go", aliased).len(), 1);
}

#[test]
fn a_dart_service_may_not_import_the_browser_libraries() {
    let source = "import 'dart:js_interop';\nimport \"package:web/web.dart\";\nimport 'dart:html' as html;\nimport 'dart:math';\n\n@Service()\nclass PlaceService {}\n";
    let found = found_text("client/services/PlaceService.dart", source);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found[0].starts_with("Remove `dart:js_interop`."),
        "{found:?}"
    );
    assert!(found[1].starts_with("Remove `package:web/`."), "{found:?}");
    assert!(found[2].starts_with("Remove `dart:html`."), "{found:?}");
}

#[test]
fn a_kotlin_service_may_not_use_javascript_interop() {
    let source = "import kotlinx.browser.window\n\n@JsFun(\"() => location.href\")\nexternal fun where(): String\n\n@Service\nclass PlaceService {\n    val page = js(\"document.title\")\n    val external_count = 0\n    // external JsFun\n}\n";
    let found = found_text("client/services/PlaceService.kt", source);
    assert_eq!(found.len(), 4, "{found:?}");
    assert!(
        found[0].starts_with("Remove `kotlinx.browser`."),
        "{found:?}"
    );
    assert!(found[1].starts_with("Remove `JsFun`."), "{found:?}");
    assert!(found[2].starts_with("Remove `external`."), "{found:?}");
    assert!(found[3].starts_with("Remove `js(`."), "{found:?}");
    let problems = refused("client/services/PlaceService.kt", source);
    assert!(problems[0].starts_with("TY1317 Kotlin services cannot call browser APIs."));
}

#[test]
fn a_swift_service_may_not_use_javascript_interop() {
    let source = "import JavaScriptKit\n\n@_extern(wasm, module: \"env\", name: \"where\")\nfunc whereAmI() -> Double\n\n@Service\nfinal class PlaceService {\n    let page = JSObject.global.document\n    let JSValueCount = 0\n    // JavaScriptKit JSObject\n}\n";
    let found = found_text("client/services/PlaceService.swift", source);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found[0].starts_with("Remove `JavaScriptKit`."), "{found:?}");
    assert!(found[1].starts_with("Remove `_extern`."), "{found:?}");
    assert!(found[2].starts_with("Remove `JSObject`."), "{found:?}");
    let problems = refused("client/services/PlaceService.swift", source);
    assert!(problems[0].starts_with("TY1317 Swift services cannot call browser APIs."));
}

#[test]
fn a_java_service_may_not_use_teavm_javascript_interop() {
    let source = "import org.teavm.jso.JSBody;\n\n@Service\nclass PlaceService {\n    @JSBody(script = \"return location.href;\")\n    static native String where();\n    int nativeCount = 0;\n    // JSObject JSExport\n}\n";
    let found = found_text("client/services/PlaceService.java", source);
    assert_eq!(found.len(), 4, "{found:?}");
    assert!(found[0].starts_with("Remove `org.teavm`."), "{found:?}");
    assert!(found[1].starts_with("Remove `JSBody`."), "{found:?}");
    assert!(found[2].starts_with("Remove `JSBody`."), "{found:?}");
    assert!(found[3].starts_with("Remove `native`."), "{found:?}");
    let problems = refused("client/services/PlaceService.java", source);
    assert!(problems[0].starts_with("TY1317 Java services cannot call browser APIs."));
}

#[test]
fn a_csharp_service_may_not_use_javascript_interop() {
    let source = "using System.Runtime.InteropServices.JavaScript;\n\n[Service]\npublic partial class PlaceService\n{\n    [JSImport(\"navigator.geolocation\", \"main.js\")]\n    internal static partial string Where();\n}\n";
    let found = found_text("client/services/PlaceService.cs", source);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].starts_with("Remove `InteropServices.JavaScript`."),
        "{found:?}"
    );
    assert!(found[1].starts_with("Remove `JSImport`."), "{found:?}");
}

#[test]
fn an_fsharp_service_may_not_use_javascript_interop() {
    let source = "open System.Runtime.InteropServices.JavaScript\n\n[<Service>]\ntype PlaceService() =\n    [<JSImport(\"navigator.geolocation\", \"main.js\")>]\n    static member Where() : string = \"\"\n    // JSExport stays in a comment\n";
    let problems = refused("client/services/PlaceService.fs", source);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(
        problems[0].starts_with("TY1317 F# services cannot call browser APIs."),
        "{problems:?}"
    );
}

#[test]
fn a_rust_service_may_not_use_browser_crates_or_import_from_the_page() {
    let source = "use wasm_bindgen::prelude::*;\n\n#[link(wasm_import_module = \"env\")]\nunsafe extern \"C\" {\n    fn where_am_i() -> f64;\n}\n\nfn window() {\n    let _ = web_sys::window();\n}\n";
    let found = found_text("client/services/PlaceService.rs", source);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        refused("client/services/PlaceService.rs", source)[0]
            .contains("Rust services cannot call browser APIs")
    );
}

#[test]
fn a_ruby_service_may_not_call_the_page_glue() {
    let source = "#[Service]\nclass PlaceService\n  def where\n    TachyonWeb.host_emit(\"x\")\n    JS.global\n  end\nend\n";
    let found = found_text("client/services/PlaceService.rb", source);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        refused("client/services/PlaceService.rb", source)[0]
            .contains("Ruby services cannot call browser APIs")
    );
}

#[test]
fn comments_strings_and_longer_names_are_not_interop() {
    let cases = [
        (
            "client/services/NoteService.go",
            "// import \"syscall/js\"\nvar text = \"see syscall/js\"\n",
        ),
        (
            "client/services/NoteService.dart",
            "// import 'dart:html';\nfinal text = 'dart:htmlx';\n",
        ),
        (
            "client/services/NoteService.cs",
            "// [JSImport]\nvar text = \"JSImport\";\nvar MyJSImporter = 1;\n",
        ),
        (
            "client/services/NoteService.rs",
            "// use wasm_bindgen;\nlet text = \"web_sys\";\nlet my_js_sys_count = 1;\n",
        ),
        (
            "client/services/NoteService.rb",
            "# TachyonWeb\ntext = \"JS.global\"\nMYJS = 1\n",
        ),
    ];
    for (relative, source) in cases {
        assert!(refused(relative, source).is_empty(), "{relative}");
    }
}

#[test]
fn server_code_and_other_languages_are_not_checked() {
    let go = "import \"syscall/js\"\n";
    assert!(refused("server/services/PlaceService.go", go).is_empty());
    let javascript = "import 'dart:html'\n";
    assert!(refused("client/services/PlaceService.js", javascript).is_empty());
}

#[test]
fn a_captured_file_with_interop_fails_the_check() {
    let source = b"import \"syscall/js\"\n";
    let failure = BrowserInteropCheck::check_file(Path::new("client/models/Place.go"), source)
        .expect_err("refused")
        .to_string();
    assert!(failure.contains("TY1317"), "{failure}");
    let clean = b"import \"time\"\n";
    assert!(BrowserInteropCheck::check_file(Path::new("client/models/Place.go"), clean).is_ok());
}

#[test]
fn project_discovery_refuses_a_web_service_that_calls_the_browser() {
    let root = tempfile::tempdir().expect("project");
    let files = [
        ("client/pages/tac.html", "<p>Where</p>\n"),
        (
            "client/services/PlaceService.go",
            "import \"syscall/js\"\n\n// @Service\ntype PlaceService struct {\n\tLabel string\n}\n\nfunc (place *PlaceService) Title() string {\n\treturn js.Global().Get(\"document\").Get(\"title\").String()\n}\n",
        ),
    ];
    for (relative, text) in files {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, text).expect("write");
    }
    let failure = crate::ProjectDiscovery::discover(root.path())
        .expect_err("browser interop is refused")
        .to_string();
    assert!(failure.contains("TY1317"), "{failure}");
    assert!(
        failure.contains("Go services cannot call browser APIs"),
        "{failure}"
    );
}
