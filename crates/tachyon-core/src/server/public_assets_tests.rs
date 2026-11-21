use super::PublicAssets;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

struct SeparateDeploy;

impl SeparateDeploy {
    const WORKER: &'static str =
        "const ASSETS = {\"/index.html\":{\"sha256\":\"a\",\"bytes\":1}}\n";
    const LIST: &'static str = r#"{"apps":["/chat"]}"#;
    const TABLE: &'static str = r#"{"tachyon":"1","version":"ab","assets":{"/chat/.tachyon/tac-client.js":{"sha256":"b","bytes":2}}}"#;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        fs::write(path, contents).expect("file");
    }

    fn create(list: &str) -> TempDir {
        let root = tempfile::tempdir().expect("site");
        Self::write(root.path(), "tachyon-sw.js", Self::WORKER);
        Self::write(root.path(), ".tachyon/apps.json", list);
        Self::write(root.path(), "chat/.tachyon/app.json", Self::TABLE);
        root
    }
}

#[test]
fn a_shell_file_stays_public() {
    let root = SeparateDeploy::create(SeparateDeploy::LIST);
    assert!(PublicAssets::read(root.path()).lists("/index.html"));
}

#[test]
fn a_file_of_a_separately_deployed_app_is_public() {
    let root = SeparateDeploy::create(SeparateDeploy::LIST);
    assert!(PublicAssets::read(root.path()).lists("/chat/.tachyon/tac-client.js"));
}

#[test]
fn an_app_missing_from_the_list_is_not_public() {
    let root = SeparateDeploy::create(r#"{"apps":[]}"#);
    assert!(!PublicAssets::read(root.path()).lists("/chat/.tachyon/tac-client.js"));
}

#[test]
fn a_bad_prefix_in_the_list_is_ignored() {
    let root = SeparateDeploy::create(r#"{"apps":["/../chat"]}"#);
    assert!(!PublicAssets::read(root.path()).lists("/chat/.tachyon/tac-client.js"));
}

#[test]
fn a_new_app_deploy_makes_the_table_stale() {
    let root = SeparateDeploy::create(SeparateDeploy::LIST);
    let assets = PublicAssets::read(root.path());
    SeparateDeploy::write(
        root.path(),
        "chat/.tachyon/app.json",
        r#"{"tachyon":"1","version":"cd","assets":{}}"#,
    );
    assert!(!assets.is_current(root.path()));
}

#[test]
fn an_unchanged_site_keeps_the_table_current() {
    let root = SeparateDeploy::create(SeparateDeploy::LIST);
    let assets = PublicAssets::read(root.path());
    assert!(assets.is_current(root.path()));
}
