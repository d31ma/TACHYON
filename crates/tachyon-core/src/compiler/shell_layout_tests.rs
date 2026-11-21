#![allow(clippy::expect_used)]

use super::{BuildOptions, WebCompiler};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct LayoutFixture;

impl LayoutFixture {
    const MANIFEST: &'static str = r#"{"name":"Site","tac":{"layout":"app-shell"}}"#;
    const LAYOUT: &'static str = "<div><header>Site header</header><main><outlet /></main></div>";
    const SHELL_PLAN: &'static str = r#"id="tachyon-shell""#;
    const OUTLET: &'static str = r#"{"k":"outlet"}"#;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        fs::write(path, contents).expect("file");
    }

    fn shell(root: &Path, manifest: &str, layout: &str) {
        Self::write(root, "client/manifest.json", manifest);
        Self::write(root, "client/components/app/shell/tac.html", layout);
        Self::write(root, "client/pages/tac.html", "<page-home />");
        Self::write(
            root,
            "client/components/page/home/tac.html",
            "<h1>Home page</h1>",
        );
    }

    fn create(manifest: &str, layout: &str) -> TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::shell(root.path(), manifest, layout);
        root
    }

    fn options() -> BuildOptions {
        BuildOptions {
            output_directory: PathBuf::from("dist"),
            ..BuildOptions::default()
        }
    }

    fn page(manifest: &str, layout: &str) -> String {
        let root = Self::create(manifest, layout);
        WebCompiler::build(root.path(), &Self::options()).expect("build");
        fs::read_to_string(root.path().join("dist/index.html")).expect("index page")
    }

    fn failure(root: &Path) -> String {
        WebCompiler::build(root, &Self::options())
            .expect_err("refused")
            .to_string()
    }

    fn refused(manifest: &str, layout: &str) -> String {
        let root = Self::create(manifest, layout);
        Self::failure(root.path())
    }
}

#[test]
fn a_page_document_holds_the_shell_plan() {
    let page = LayoutFixture::page(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    assert!(page.contains(LayoutFixture::SHELL_PLAN), "{page}");
}

#[test]
fn the_shell_plan_holds_the_layout_template() {
    let page = LayoutFixture::page(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    assert!(page.contains("Site header"), "{page}");
}

#[test]
fn the_document_holds_one_outlet() {
    let page = LayoutFixture::page(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    assert_eq!(page.matches(LayoutFixture::OUTLET).count(), 1, "{page}");
}

#[test]
fn the_page_plan_still_holds_the_page() {
    let page = LayoutFixture::page(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    assert!(page.contains("Home page"), "{page}");
}

#[test]
fn a_site_without_a_layout_writes_no_shell_plan() {
    let page = LayoutFixture::page(
        r#"{"name":"Site"}"#,
        LayoutFixture::LAYOUT.replace("<outlet />", "").as_str(),
    );
    assert!(!page.contains(LayoutFixture::SHELL_PLAN), "{page}");
}

#[test]
fn a_layout_without_an_outlet_is_refused() {
    let error = LayoutFixture::refused(LayoutFixture::MANIFEST, "<main>No outlet</main>");
    assert!(error.contains("exactly one <outlet />"), "{error}");
}

#[test]
fn a_layout_with_two_outlets_is_refused() {
    let error = LayoutFixture::refused(
        LayoutFixture::MANIFEST,
        "<main><outlet /></main><aside><outlet /></aside>",
    );
    assert!(error.contains("exactly one <outlet />"), "{error}");
}

#[test]
fn an_unknown_layout_is_refused() {
    let error = LayoutFixture::refused(
        r#"{"name":"Site","tac":{"layout":"site-frame"}}"#,
        LayoutFixture::LAYOUT,
    );
    assert!(error.contains("not a component of this project"), "{error}");
}

#[test]
fn a_page_with_an_outlet_is_refused() {
    let root = LayoutFixture::create(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    LayoutFixture::write(
        root.path(),
        "client/components/page/home/tac.html",
        "<main><outlet /></main>",
    );
    let error = LayoutFixture::failure(root.path());
    assert!(error.contains("holds <outlet />"), "{error}");
}

#[test]
fn an_outlet_with_attributes_is_refused() {
    let error = LayoutFixture::refused(
        LayoutFixture::MANIFEST,
        r#"<main><outlet name="page" /></main>"#,
    );
    assert!(error.contains("outlet"), "{error}");
}

#[test]
fn a_page_app_cannot_declare_a_layout() {
    let error = LayoutFixture::refused(
        r#"{"name":"Chat","tac":{"mount":"/chat","layout":"app-shell"}}"#,
        LayoutFixture::LAYOUT,
    );
    assert!(error.contains("cannot declare tac.layout"), "{error}");
}

#[test]
fn an_app_page_of_the_bundle_holds_the_shell_plan() {
    let root = tempfile::tempdir().expect("site");
    LayoutFixture::shell(
        &root.path().join("home"),
        r#"{"name":"Site","tac":{"layout":"app-shell","apps":{"/chat":"../chat"}}}"#,
        LayoutFixture::LAYOUT,
    );
    let chat = root.path().join("chat");
    LayoutFixture::write(
        &chat,
        "client/manifest.json",
        r#"{"name":"Chat","tac":{"mount":"/chat"}}"#,
    );
    LayoutFixture::write(&chat, "client/pages/tac.html", "<chat-home />");
    LayoutFixture::write(
        &chat,
        "client/components/chat/home/tac.html",
        "<h1>Chat page</h1>",
    );
    WebCompiler::build(root.path().join("home"), &LayoutFixture::options()).expect("bundle");
    let page =
        fs::read_to_string(root.path().join("home/dist/chat/index.html")).expect("chat page");
    assert!(page.contains(LayoutFixture::SHELL_PLAN), "{page}");
}

struct ShellFileFixture;

impl ShellFileFixture {
    const FILE: &'static str = "dist/.tachyon/shell.json";
    const SCRIPT_START: &'static str = r#"<script id="tachyon-shell" type="application/json">"#;

    fn inline_text(page: &str) -> String {
        let start = page.find(Self::SCRIPT_START).expect("shell script") + Self::SCRIPT_START.len();
        let length = page[start..].find("</script>").expect("script end");
        page[start..start + length].to_string()
    }

    fn built(root: &Path) -> (String, String) {
        WebCompiler::build(root, &LayoutFixture::options()).expect("build");
        let page = fs::read_to_string(root.join("dist/index.html")).expect("index page");
        let file = fs::read_to_string(root.join(Self::FILE)).expect("shell file");
        (Self::inline_text(&page), file)
    }

    fn plan(file: &str) -> serde_json::Value {
        serde_json::from_str(file).expect("shell plan")
    }
}

#[test]
fn the_shell_file_holds_the_same_plan_as_the_page() {
    let root = LayoutFixture::create(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    let (inline, file) = ShellFileFixture::built(root.path());
    assert_eq!(inline, file);
}

#[test]
fn the_shell_file_names_no_styles_without_component_styles() {
    let root = LayoutFixture::create(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    let (_, file) = ShellFileFixture::built(root.path());
    assert_eq!(
        ShellFileFixture::plan(&file)["styles"],
        serde_json::json!([])
    );
}

#[test]
fn the_shell_file_names_the_tachyon_version() {
    let root = LayoutFixture::create(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    let (_, file) = ShellFileFixture::built(root.path());
    assert_eq!(
        ShellFileFixture::plan(&file)["tachyon"],
        serde_json::json!(env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn the_shell_file_names_the_component_styles() {
    let root = LayoutFixture::create(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    LayoutFixture::write(
        root.path(),
        "client/components/app/shell/tac.css",
        "header { color: navy; }",
    );
    let (_, file) = ShellFileFixture::built(root.path());
    assert_eq!(
        ShellFileFixture::plan(&file)["styles"],
        serde_json::json!(["/.tachyon/components.css"])
    );
}

#[test]
fn a_site_without_a_layout_writes_no_shell_file() {
    let root = LayoutFixture::create(
        r#"{"name":"Site"}"#,
        LayoutFixture::LAYOUT.replace("<outlet />", "").as_str(),
    );
    WebCompiler::build(root.path(), &LayoutFixture::options()).expect("build");
    assert!(!root.path().join(ShellFileFixture::FILE).exists());
}

#[test]
fn the_shell_only_build_writes_the_shell_file() {
    let root = tempfile::tempdir().expect("site");
    let home = root.path().join("home");
    LayoutFixture::shell(
        &home,
        r#"{"name":"Site","tac":{"layout":"app-shell","apps":{"/chat":"../chat"}}}"#,
        LayoutFixture::LAYOUT,
    );
    LayoutFixture::write(
        &root.path().join("chat"),
        "client/manifest.json",
        r#"{"name":"Chat","tac":{"mount":"/chat"}}"#,
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime
        .block_on(WebCompiler::build_shell_async(
            &home,
            &LayoutFixture::options(),
        ))
        .expect("shell build");
    assert!(home.join(ShellFileFixture::FILE).exists());
}

struct ShellScriptFixture;

impl ShellScriptFixture {
    const FILE: &'static str = "dist/.tachyon/tac-shell.js";
    const PRELOAD: &'static str =
        r#"<link rel="modulepreload" href="/.tachyon/tac-shell.js" data-tachyon-runtime>"#;
}

#[test]
fn a_shell_build_writes_the_shell_script() {
    let root = LayoutFixture::create(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    WebCompiler::build(root.path(), &LayoutFixture::options()).expect("build");
    let script = fs::read_to_string(root.path().join(ShellScriptFixture::FILE)).expect("script");
    assert!(script.contains("export class ShellApp"), "{script}");
}

#[test]
fn a_shell_page_preloads_the_shell_script() {
    let page = LayoutFixture::page(LayoutFixture::MANIFEST, LayoutFixture::LAYOUT);
    assert!(page.contains(ShellScriptFixture::PRELOAD), "{page}");
}

#[test]
fn a_site_without_a_layout_has_no_shell_script() {
    let root = LayoutFixture::create(
        r#"{"name":"Site"}"#,
        LayoutFixture::LAYOUT.replace("<outlet />", "").as_str(),
    );
    WebCompiler::build(root.path(), &LayoutFixture::options()).expect("build");
    let page = fs::read_to_string(root.path().join("dist/index.html")).expect("index page");
    assert!(!root.path().join(ShellScriptFixture::FILE).exists());
    assert!(!page.contains("tac-shell.js"), "{page}");
}

#[test]
fn the_runtime_holds_no_soft_navigation_code() {
    let runtime = crate::template::ClientViewRenderer::shipped_runtime();
    assert!(!runtime.contains("SoftNavigation"));
    assert!(runtime.contains("/.tachyon/tac-shell.js"));
}
