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
