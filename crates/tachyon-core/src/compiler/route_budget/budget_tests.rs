use super::BudgetCheck;
use std::path::{Path, PathBuf};

fn project(manifest: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("temporary project");
    std::fs::create_dir_all(root.path().join("client")).expect("client folder");
    std::fs::write(root.path().join("client/manifest.json"), manifest).expect("manifest");
    root
}

fn file(path: &str, text: &str, padding: usize) -> (PathBuf, Vec<u8>) {
    let mut bytes = text.as_bytes().to_vec();
    bytes.resize(bytes.len() + padding, b' ');
    (PathBuf::from(path), bytes)
}

fn site() -> Vec<(PathBuf, Vec<u8>)> {
    vec![
        file(
            "index.html",
            r#"<script type="module" src="/.tachyon/tac-client.js"></script><script type="application/json">{"services":[{"module":"/.tachyon/services/ShopService.js"}]}</script>"#,
            0,
        ),
        file(
            "about/index.html",
            r#"<script type="module" src="/.tachyon/tac-client.js"></script>"#,
            0,
        ),
        file(
            ".tachyon/tac-client.js",
            "navigator.serviceWorker.register('/tachyon-sw.js')",
            1024,
        ),
        file(
            ".tachyon/services/ShopService.js",
            "import { activate } from './tachyon-go.js'",
            0,
        ),
        file(
            ".tachyon/services/tachyon-go.js",
            "fetch(new URL('./go/services.wasm?v=1', import.meta.url)); import('../unused/../tac-client.js')",
            2048,
        ),
        file(".tachyon/services/go/services.wasm", "", 3 * 1024),
        file("tachyon-sw.js", "", 50 * 1024),
        file("shared/big.js", "", 90 * 1024),
    ]
}

fn messages(manifest: &str) -> Vec<String> {
    let root = project(manifest);
    BudgetCheck::verify(root.path(), &site(), true, &|_| None)
        .err()
        .map(|failure| {
            failure
                .diagnostics()
                .iter()
                .map(|problem| format!("{} {}", problem.code, problem.message))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_route_over_its_script_or_web_assembly_budget_fails_the_build() {
    let found = messages(r#"{"tac":{"budgets":{"check":true,"script":3,"webassembly":2}}}"#);
    assert_eq!(
        found,
        [
            "TY1337 Route '/' loads 4 KB of script, and its budget is 3 KB.",
            "TY1337 Route '/' loads 3 KB of WebAssembly, and its budget is 2 KB.",
        ]
    );
}

#[test]
fn the_help_lists_the_biggest_files_first() {
    let root = project(r#"{"tac":{"budgets":{"check":true,"script":2}}}"#);
    let failure =
        BudgetCheck::verify(root.path(), &site(), true, &|_| None).expect_err("over budget");
    let help = failure.diagnostics()[0].help.clone().expect("help");
    assert!(
        help.starts_with("The biggest files: /.tachyon/services/tachyon-go.js (3 KB), /.tachyon/tac-client.js (2 KB), /.tachyon/services/ShopService.js (1 KB)."),
        "{help}"
    );
    assert!(help.contains("tac.budgets.script"), "{help}");
}

#[test]
fn routes_within_budget_a_switched_off_check_and_a_watch_build_pass() {
    assert!(messages(r#"{"tac":{"budgets":{"check":true}}}"#).is_empty());
    assert!(messages(r#"{"tac":{"budgets":{"check":false,"script":1}}}"#).is_empty());
    let root = project(r#"{"tac":{"budgets":{"check":true,"script":1}}}"#);
    assert!(BudgetCheck::verify(root.path(), &site(), false, &|_| None).is_ok());
    assert!(BudgetCheck::verify(Path::new("/missing/project"), &site(), true, &|_| None).is_ok());
}

#[test]
fn a_compressed_copy_counts_with_the_size_that_is_sent() {
    let root = project(r#"{"tac":{"budgets":{"check":true,"script":1,"webassembly":1}}}"#);
    let sent = |path: &str| {
        [
            ".tachyon/services/go/services.wasm",
            ".tachyon/services/tachyon-go.js",
        ]
        .contains(&path)
        .then_some(512)
    };
    let failure = BudgetCheck::verify(root.path(), &site(), true, &sent)
        .expect_err("the scripts are still over");
    let messages: Vec<&str> = failure
        .diagnostics()
        .iter()
        .map(|problem| problem.message.as_str())
        .collect();
    assert_eq!(
        messages,
        [
            "Route '/about' loads 2 KB of script, and its budget is 1 KB.",
            "Route '/' loads 2 KB of script, and its budget is 1 KB.",
        ]
    );
}
