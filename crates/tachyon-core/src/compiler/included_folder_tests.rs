#![allow(clippy::expect_used)]

use super::{BuildOptions, WebCompiler};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct IncludeFixture;

impl IncludeFixture {
    const MANIFEST: &'static str = r#"{"name":"App","tac":{"include":["../kit"]}}"#;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        fs::write(path, contents).expect("file");
    }

    fn create(manifest: &str) -> TempDir {
        let root = tempfile::tempdir().expect("site");
        let app = root.path().join("app");
        let kit = root.path().join("kit");
        Self::write(&app, "client/manifest.json", manifest);
        Self::write(&app, "client/pages/tac.html", "<kit-banner />");
        Self::write(
            &kit,
            "client/components/kit/banner/tac.html",
            "<main><h1>Kit banner {kitBanner.ready}</h1></main>",
        );
        Self::write(
            &kit,
            "client/services/KitBannerService.js",
            "@Service()\nexport default class KitBannerService { ready = 1 }\n",
        );
        Self::write(&kit, "client/shared/kit.css", "main { margin: 0 }\n");
        root
    }

    fn options() -> BuildOptions {
        BuildOptions {
            output_directory: PathBuf::from("dist"),
            ..BuildOptions::default()
        }
    }

    fn built(root: &TempDir) {
        WebCompiler::build(root.path().join("app"), &Self::options()).expect("build");
    }

    fn failure(root: &TempDir) -> String {
        WebCompiler::build(root.path().join("app"), &Self::options())
            .expect_err("refused")
            .to_string()
    }

    fn output(root: &TempDir, relative: &str) -> PathBuf {
        root.path().join("app/dist").join(relative)
    }
}

#[test]
fn a_page_renders_a_component_from_the_included_folder() {
    let root = IncludeFixture::create(IncludeFixture::MANIFEST);
    IncludeFixture::built(&root);
    let page = fs::read_to_string(IncludeFixture::output(&root, "index.html")).expect("index page");
    assert!(page.contains("Kit banner"), "{page}");
}

#[test]
fn a_service_from_the_included_folder_is_built() {
    let root = IncludeFixture::create(IncludeFixture::MANIFEST);
    IncludeFixture::built(&root);
    assert!(IncludeFixture::output(&root, ".tachyon/services/KitBannerService.js").is_file());
}

#[test]
fn a_shared_file_from_the_included_folder_is_copied() {
    let root = IncludeFixture::create(IncludeFixture::MANIFEST);
    IncludeFixture::built(&root);
    assert!(IncludeFixture::output(&root, "shared/kit.css").is_file());
}

#[test]
fn the_included_folder_is_not_changed_by_the_build() {
    let root = IncludeFixture::create(IncludeFixture::MANIFEST);
    IncludeFixture::built(&root);
    assert!(!root.path().join("kit/dist").exists());
}

#[test]
fn a_file_in_both_the_app_and_the_included_folder_is_refused() {
    let root = IncludeFixture::create(IncludeFixture::MANIFEST);
    IncludeFixture::write(
        &root.path().join("app"),
        "client/shared/kit.css",
        "main {}\n",
    );
    let error = IncludeFixture::failure(&root);
    assert!(error.contains("both have client/shared/kit.css"), "{error}");
}

#[test]
fn an_included_folder_with_pages_is_refused() {
    let root = IncludeFixture::create(IncludeFixture::MANIFEST);
    IncludeFixture::write(
        &root.path().join("kit"),
        "client/pages/tac.html",
        "<main></main>",
    );
    let error = IncludeFixture::failure(&root);
    assert!(error.contains("has client/pages"), "{error}");
}

#[test]
fn an_included_folder_with_server_code_is_refused() {
    let root = IncludeFixture::create(IncludeFixture::MANIFEST);
    IncludeFixture::write(&root.path().join("kit"), "server/routes/GET.js", "");
    let error = IncludeFixture::failure(&root);
    assert!(error.contains("has server"), "{error}");
}

#[test]
fn an_absolute_included_folder_is_refused() {
    #[cfg(unix)]
    let manifest = r#"{"name":"App","tac":{"include":["/tmp/kit"]}}"#;
    #[cfg(windows)]
    let manifest = r#"{"name":"App","tac":{"include":["C:/tmp/kit"]}}"#;
    let root = IncludeFixture::create(manifest);
    let error = IncludeFixture::failure(&root);
    assert!(error.contains("relative to the project folder"), "{error}");
}

#[test]
fn an_included_folder_inside_the_project_is_refused() {
    let root = IncludeFixture::create(r#"{"name":"App","tac":{"include":["client"]}}"#);
    let error = IncludeFixture::failure(&root);
    assert!(error.contains("inside the project or holds it"), "{error}");
}

#[test]
fn a_missing_included_folder_is_refused() {
    let root = IncludeFixture::create(r#"{"name":"App","tac":{"include":["../none"]}}"#);
    let error = IncludeFixture::failure(&root);
    assert!(error.contains("cannot be opened"), "{error}");
}
