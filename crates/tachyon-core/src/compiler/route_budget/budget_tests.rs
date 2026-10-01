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

#[test]
fn an_explicit_unused_python_profile_does_not_raise_an_ordinary_route_budget() {
    let root = project(r#"{"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}}"#);
    let files = vec![
        file("index.html", "<script src='./ordinary.js'></script>", 0),
        file("ordinary.js", "", 201 * 1024),
        file(crate::tac_python::RuntimeInventory::ENGINE, "", 0),
    ];
    let failure = BudgetCheck::verify(root.path(), &files, true, &|_| None)
        .err()
        .unwrap_or_else(|| panic!("ordinary script default"));
    assert!(failure.to_string().contains("budget is 200 KB"));
}

#[test]
fn python_script_headroom_preserves_explicit_lower_limits_and_wasm_default() {
    let ordinary = project("{}");
    let selected = super::RouteBudgetSettings::read(ordinary.path()).for_python_route(true);
    assert_eq!(selected.script.bytes, 512 * 1024);
    assert_eq!(selected.web_assembly.bytes, 3584 * 1024);
    let lower = project(r#"{"tac":{"budgets":{"script":100}}}"#);
    assert_eq!(
        super::RouteBudgetSettings::read(lower.path())
            .for_python_route(true)
            .script
            .bytes,
        100 * 1024
    );
}

#[test]
fn missing_python_runtime_dependencies_fail_without_bypassing_route_checks() {
    let root = project(r#"{"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}}"#);
    let files = vec![
        file(
            "index.html",
            &format!(
                "<script src='/{}'></script>",
                crate::tac_python::RuntimeInventory::ENGINE
            ),
            0,
        ),
        file(crate::tac_python::RuntimeInventory::ENGINE, "", 0),
    ];
    assert!(
        BudgetCheck::verify(root.path(), &files, true, &|_| None)
            .err()
            .unwrap_or_else(|| panic!("missing runtime"))
            .to_string()
            .contains("not emitted")
    );
}

#[test]
#[ignore = "requires pinned five-file core assets; set TAC_PYTHON_CORE to the verified local directory"]
fn real_runtime_reports_auxiliary_bytes_and_keeps_the_wasm_transfer_budget() {
    let root = project(r#"{"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}}"#);
    let asset_root =
        std::env::var_os("TAC_PYTHON_CORE").unwrap_or_else(|| panic!("explicit runtime assets"));
    let names = [
        "pyodide.mjs",
        "pyodide.asm.mjs",
        "pyodide.asm.wasm",
        "python_stdlib.zip",
        "pyodide-lock.json",
    ];
    let mut files: Vec<(PathBuf, Vec<u8>)> = names
        .iter()
        .map(|name| {
            (
                PathBuf::from(format!(".tachyon/services/python/314.0.7/{name}")),
                std::fs::read(Path::new(&asset_root).join(name))
                    .unwrap_or_else(|error| panic!("pinned asset: {error:?}")),
            )
        })
        .collect();
    files.push(file(
        "index.html",
        &format!(
            "<script src='/{}'></script>",
            crate::tac_python::RuntimeInventory::ENGINE
        ),
        0,
    ));
    files.push(file(crate::tac_python::RuntimeInventory::ENGINE, "", 0));
    let failure = BudgetCheck::verify(root.path(), &files, true, &|_| None)
        .err()
        .unwrap_or_else(|| panic!("raw Wasm is over unchanged limit"));
    assert!(failure.to_string().contains("budget is 3584 KB"));
    let proxy_sent = |path: &str| match path.rsplit('/').next().unwrap_or_default() {
        "pyodide.mjs" => Some(6453),
        "pyodide.asm.mjs" => Some(207_390),
        "pyodide.asm.wasm" => Some(2_741_061),
        "pyodide-lock.json" => Some(21689),
        _ => None,
    };
    let notes = BudgetCheck::check(root.path(), &files, true, &proxy_sent)
        .unwrap_or_else(|error| panic!("proxy size accounting: {error:?}"));
    assert!(notes[0].contains("5522230 bytes"));
    assert!(notes[0].contains("2567326 auxiliary ZIP/lock bytes"));
}
