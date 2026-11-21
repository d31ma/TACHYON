#![allow(clippy::expect_used)]

use super::{BuildOptions, WebCompiler};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct SiteFixture;

impl SiteFixture {
    const SHELL_MANIFEST: &'static str =
        r#"{"name":"Site","start_url":"/","tac":{"apps":{"/chat":"../chat","/mail":"../mail"}}}"#;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        fs::write(path, contents).expect("file");
    }

    fn page(root: &Path, page: &str, name: &str) {
        Self::write(
            root,
            &format!("client/pages/{page}tac.html"),
            &format!("<site-{name} />"),
        );
        Self::write(
            root,
            &format!("client/components/site/{name}/tac.html"),
            &format!("<main><h1>{name}</h1></main>"),
        );
    }

    fn app(root: &Path, folder: &str, mount: &str) {
        let app = root.join(folder);
        Self::write(
            &app,
            "client/manifest.json",
            &format!(r#"{{"name":"{folder}","tac":{{"mount":"{mount}"}}}}"#),
        );
        Self::page(&app, "", folder);
        Self::page(&app, "_id/", &format!("{folder}-item"));
    }

    fn create(shell_manifest: &str) -> TempDir {
        let root = tempfile::tempdir().expect("site");
        let shell = root.path().join("home");
        Self::write(&shell, "client/manifest.json", shell_manifest);
        Self::page(&shell, "", "home");
        Self::app(root.path(), "chat", "/chat");
        Self::app(root.path(), "mail", "/mail");
        root
    }

    fn options() -> BuildOptions {
        BuildOptions {
            output_directory: PathBuf::from("dist"),
            ..BuildOptions::default()
        }
    }

    fn bundle() -> TempDir {
        let root = Self::create(Self::SHELL_MANIFEST);
        WebCompiler::build(root.path().join("home"), &Self::options()).expect("bundle");
        root
    }

    fn shell_only() -> TempDir {
        let root = Self::create(Self::SHELL_MANIFEST);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime
            .block_on(WebCompiler::build_shell_async(
                root.path().join("home"),
                &Self::options(),
            ))
            .expect("shell");
        root
    }

    fn failure(shell_manifest: &str) -> String {
        let root = Self::create(shell_manifest);
        WebCompiler::build(root.path().join("home"), &Self::options())
            .expect_err("refused")
            .to_string()
    }

    fn output(root: &TempDir, relative: &str) -> PathBuf {
        root.path().join("home/dist").join(relative)
    }

    fn read(root: &TempDir, relative: &str) -> String {
        fs::read_to_string(Self::output(root, relative)).expect(relative)
    }
}

#[test]
fn the_bundle_holds_the_shell_page() {
    let root = SiteFixture::bundle();
    assert!(SiteFixture::output(&root, "index.html").is_file());
}

#[test]
fn the_bundle_holds_each_app_page() {
    let root = SiteFixture::bundle();
    assert!(SiteFixture::output(&root, "mail/_id/index.html").is_file());
}

#[test]
fn the_bundle_holds_each_app_runtime() {
    let root = SiteFixture::bundle();
    assert!(SiteFixture::output(&root, "chat/.tachyon/tac-client.js").is_file());
}

#[test]
fn an_app_page_of_the_bundle_loads_its_runtime_from_its_prefix() {
    let root = SiteFixture::bundle();
    let page = SiteFixture::read(&root, "chat/index.html");
    assert!(
        page.contains(r#"src="/chat/.tachyon/tac-client.js""#),
        "{page}"
    );
}

#[test]
fn the_service_worker_knows_every_app_file() {
    let root = SiteFixture::bundle();
    let worker = SiteFixture::read(&root, "tachyon-sw.js");
    assert!(worker.contains("/chat/.tachyon/tac-client.js"), "{worker}");
}

#[test]
fn the_route_manifest_lists_every_app_page() {
    let root = SiteFixture::bundle();
    let manifest = SiteFixture::read(&root, "route-manifest.json");
    assert!(manifest.contains(r#""/mail/_id""#), "{manifest}");
}

#[test]
fn the_bundle_lists_the_page_apps() {
    let root = SiteFixture::bundle();
    let list: serde_json::Value =
        serde_json::from_str(&SiteFixture::read(&root, ".tachyon/apps.json")).expect("list");
    assert_eq!(list, serde_json::json!({ "apps": ["/chat", "/mail"] }));
}

#[test]
fn an_app_asset_table_is_not_copied_into_the_bundle() {
    let root = SiteFixture::bundle();
    assert!(!SiteFixture::output(&root, "chat/.tachyon/app.json").exists());
}

#[test]
fn the_shell_writes_no_asset_table_of_its_own() {
    let root = SiteFixture::shell_only();
    assert!(!SiteFixture::output(&root, ".tachyon/app.json").exists());
}

#[test]
fn an_app_route_manifest_is_not_copied_into_the_bundle() {
    let root = SiteFixture::bundle();
    assert!(!SiteFixture::output(&root, "chat/route-manifest.json").exists());
}

#[test]
fn the_shell_alone_writes_no_app_page() {
    let root = SiteFixture::shell_only();
    assert!(!SiteFixture::output(&root, "chat/index.html").exists());
}

#[test]
fn the_shell_alone_still_lists_the_page_apps() {
    let root = SiteFixture::shell_only();
    assert!(SiteFixture::output(&root, ".tachyon/apps.json").is_file());
}

#[test]
fn an_app_with_another_mount_is_refused() {
    let error = SiteFixture::failure(r#"{"name":"Site","tac":{"apps":{"/talk":"../chat"}}}"#);
    assert!(error.contains("Both must name the same prefix"), "{error}");
}

#[test]
fn nested_prefixes_are_refused() {
    let error = SiteFixture::failure(
        r#"{"name":"Site","tac":{"apps":{"/chat":"../chat","/chat/old":"../mail"}}}"#,
    );
    assert!(error.contains("cannot nest"), "{error}");
}

#[test]
fn an_app_folder_inside_the_shell_is_refused() {
    let error = SiteFixture::failure(r#"{"name":"Site","tac":{"apps":{"/chat":"client"}}}"#);
    assert!(error.contains("inside the shell's project"), "{error}");
}

#[test]
fn an_absolute_app_folder_is_refused() {
    let error = SiteFixture::failure(r#"{"name":"Site","tac":{"apps":{"/chat":"/tmp/chat"}}}"#);
    assert!(error.contains("relative to the project folder"), "{error}");
}

#[test]
fn a_bad_app_prefix_is_refused() {
    let error = SiteFixture::failure(r#"{"name":"Site","tac":{"apps":{"chat":"../chat"}}}"#);
    assert!(error.contains("TY1601"), "{error}");
}
