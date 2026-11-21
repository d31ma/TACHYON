#![allow(clippy::expect_used)]

use super::{BuildOptions, WebCompiler};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct MountedProject;

impl MountedProject {
    const MANIFEST: &'static str = r#"{"name":"Chat","start_url":"/chat","tac":{"mount":"/chat"}}"#;
    const PUSH_MANIFEST: &'static str =
        r#"{"name":"Chat","tac":{"mount":"/chat","browserApis":["push"]}}"#;
    const BAD_MANIFEST: &'static str = r#"{"name":"Chat","tac":{"mount":"/Chat/"}}"#;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        fs::write(path, contents).expect("file");
    }

    fn create(manifest: &str) -> TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::write(root.path(), "client/manifest.json", manifest);
        Self::write(root.path(), "client/pages/tac.html", "<page-home />");
        Self::write(
            root.path(),
            "client/components/page/home/tac.html",
            "<main><h1>Chat {pageHome.ready}</h1></main>",
        );
        Self::write(
            root.path(),
            "client/services/PageHomeService.js",
            "@Service()\nexport default class PageHomeService { ready = 1 }\n",
        );
        Self::write(root.path(), "client/pages/_id/tac.html", "<page-item />");
        Self::write(
            root.path(),
            "client/components/page/item/tac.html",
            "<main><h1>Item</h1></main>",
        );
        Self::write(root.path(), "client/shared/logo.svg", "<svg></svg>");
        root
    }

    fn options() -> BuildOptions {
        BuildOptions {
            output_directory: PathBuf::from("dist"),
            ..BuildOptions::default()
        }
    }

    fn built() -> TempDir {
        let root = Self::create(Self::MANIFEST);
        WebCompiler::build(root.path(), &Self::options()).expect("build");
        root
    }

    fn failure(manifest: &str) -> String {
        let root = Self::create(manifest);
        WebCompiler::build(root.path(), &Self::options())
            .expect_err("refused")
            .to_string()
    }

    fn output(root: &TempDir, relative: &str) -> PathBuf {
        root.path().join("dist").join(relative)
    }

    fn read(root: &TempDir, relative: &str) -> String {
        fs::read_to_string(Self::output(root, relative)).expect(relative)
    }

    fn table(root: &TempDir) -> serde_json::Value {
        serde_json::from_str(&Self::read(root, "chat/.tachyon/app.json")).expect("asset table")
    }
}

#[test]
fn the_index_page_is_written_under_the_prefix() {
    let root = MountedProject::built();
    assert!(MountedProject::output(&root, "chat/index.html").is_file());
}

#[test]
fn a_dynamic_page_is_written_under_the_prefix() {
    let root = MountedProject::built();
    assert!(MountedProject::output(&root, "chat/_id/index.html").is_file());
}

#[test]
fn the_runtime_is_written_under_the_prefix() {
    let root = MountedProject::built();
    assert!(MountedProject::output(&root, "chat/.tachyon/tac-client.js").is_file());
}

#[test]
fn a_service_is_written_under_the_prefix() {
    let root = MountedProject::built();
    assert!(MountedProject::output(&root, "chat/.tachyon/services/PageHomeService.js").is_file());
}

#[test]
fn a_shared_file_is_written_under_the_prefix() {
    let root = MountedProject::built();
    assert!(MountedProject::output(&root, "chat/shared/logo.svg").is_file());
}

#[test]
fn a_mounted_app_writes_no_service_worker() {
    let root = MountedProject::built();
    assert!(!MountedProject::output(&root, "tachyon-sw.js").exists());
}

#[test]
fn a_mounted_app_writes_its_asset_table_under_the_prefix() {
    let root = MountedProject::built();
    assert!(MountedProject::output(&root, "chat/.tachyon/app.json").is_file());
}

#[test]
fn the_asset_table_names_the_tachyon_version() {
    let root = MountedProject::built();
    assert_eq!(
        MountedProject::table(&root)["tachyon"],
        serde_json::json!(env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn the_asset_table_lists_a_file_by_its_full_address() {
    let root = MountedProject::built();
    let table = MountedProject::table(&root);
    assert!(
        table["assets"]["/chat/.tachyon/tac-client.js"]["sha256"].is_string(),
        "{table}"
    );
}

#[test]
fn the_asset_table_lists_the_index_page_by_its_folder() {
    let root = MountedProject::built();
    let table = MountedProject::table(&root);
    assert!(table["assets"]["/chat/"].is_object(), "{table}");
}

#[test]
fn the_asset_table_lists_no_deploy_file() {
    let root = MountedProject::built();
    let table = MountedProject::table(&root);
    assert!(table["assets"]["/route-manifest.json"].is_null(), "{table}");
}

#[test]
fn the_asset_table_lists_no_service_worker() {
    let root = MountedProject::built();
    let table = MountedProject::table(&root);
    assert!(table["assets"]["/chat/tachyon-sw.js"].is_null(), "{table}");
}

#[test]
fn the_asset_table_version_changes_with_the_app() {
    let root = MountedProject::built();
    let first = MountedProject::table(&root)["version"].clone();
    MountedProject::write(
        root.path(),
        "client/components/page/item/tac.html",
        "<main><h1>Changed item</h1></main>",
    );
    WebCompiler::build(root.path(), &MountedProject::options()).expect("rebuild");
    assert_ne!(MountedProject::table(&root)["version"], first);
}

#[test]
fn a_mounted_app_writes_no_web_app_manifest() {
    let root = MountedProject::built();
    assert!(!MountedProject::output(&root, "manifest.webmanifest").exists());
}

#[test]
fn a_page_loads_the_runtime_from_the_prefix() {
    let root = MountedProject::built();
    let page = MountedProject::read(&root, "chat/index.html");
    assert!(
        page.contains(r#"src="/chat/.tachyon/tac-client.js""#),
        "{page}"
    );
}

#[test]
fn a_page_loads_its_services_from_the_prefix() {
    let root = MountedProject::built();
    let page = MountedProject::read(&root, "chat/index.html");
    assert!(
        page.contains(r#""module":"/chat/.tachyon/services/PageHomeService.js""#),
        "{page}"
    );
}

#[test]
fn a_page_of_a_mounted_app_registers_no_service_worker() {
    let root = MountedProject::built();
    let page = MountedProject::read(&root, "chat/index.html");
    assert!(!page.contains("register-sw.js"), "{page}");
}

#[test]
fn a_page_keeps_the_site_manifest_link() {
    let root = MountedProject::built();
    let page = MountedProject::read(&root, "chat/index.html");
    assert!(
        page.contains(r#"<link rel="manifest" href="/manifest.webmanifest">"#),
        "{page}"
    );
}

#[test]
fn the_route_manifest_names_the_full_addresses() {
    let root = MountedProject::built();
    let manifest = MountedProject::read(&root, "route-manifest.json");
    assert!(manifest.contains(r#""/chat/_id""#), "{manifest}");
}

#[test]
fn a_mounted_app_cannot_grant_push() {
    let error = MountedProject::failure(MountedProject::PUSH_MANIFEST);
    assert!(error.contains("grant push in the shell's manifest"), "{error}");
}

#[test]
fn a_bad_mount_stops_the_build() {
    let error = MountedProject::failure(MountedProject::BAD_MANIFEST);
    assert!(error.contains("TY1601"), "{error}");
}
