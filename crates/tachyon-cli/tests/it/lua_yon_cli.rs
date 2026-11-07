#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

struct LuaYonProject;

impl LuaYonProject {
    const WORKER: &'static str = "-- @Schedule({\"fixedDelaySeconds\":1,\"initialDelaySeconds\":0})\n---@class Handler\nHandler = {}\n\n---@return nil\nfunction Handler.POST()\n  local file = io.open(MARKER, \"a\")\n  file:write(\"beat\\n\")\n  file:close()\nend\n";
    const PAGE: (&'static str, &'static str) = ("client/pages/tac.html", "<app-home />\n");
    const COMPONENT: (&'static str, &'static str) =
        ("client/components/app/home/tac.html", "<main>Home</main>\n");
    const WORKER_WAIT: Duration = Duration::from_secs(20);

    fn lua_available() -> bool {
        Command::new("lua")
            .arg("-v")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tachyon-core/tests/fixtures")
            .join(name)
    }

    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).expect("folder");
        for entry in std::fs::read_dir(from).expect("fixture folder").flatten() {
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                Self::copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).expect("fixture file");
            }
        }
    }

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, text).expect("source");
    }

    fn project(fixture: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::copy(&Self::fixture(fixture), root.path());
        root
    }

    fn invoke(root: &Path, source: &str, route: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ty"))
            .args(["handler", "invoke", source, "--route", route])
            .arg("--project")
            .arg(root)
            .output()
            .expect("ty handler invoke")
    }

    fn body(output: &Output) -> serde_json::Value {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let response: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("protocol response JSON");
        serde_json::from_str(response["body"]["data"].as_str().expect("body data"))
            .expect("handler JSON body")
    }

    fn beats(marker: &Path) -> usize {
        std::fs::read_to_string(marker)
            .map(|text| text.lines().count())
            .unwrap_or_default()
    }
}

#[test]
fn a_lua_store_is_a_provider_that_runs_its_queries() {
    if !LuaYonProject::lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = LuaYonProject::project("lua-yon-store");
    let output = LuaYonProject::invoke(
        root.path(),
        "server/controllers/ProductController.lua",
        "/products",
    );
    assert_eq!(
        LuaYonProject::body(&output),
        serde_json::json!({
            "active": [{"id": 1, "status": "active"}],
            "count": 1,
            "others": [{"id": 2, "status": "retired"}],
        })
    );
}

#[test]
fn a_missing_lua_provider_points_at_the_field_that_needs_it() {
    if !LuaYonProject::lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = LuaYonProject::project("lua-yon-store");
    let controller = root.path().join("server/controllers/ProductController.lua");
    let text = std::fs::read_to_string(&controller).expect("controller");
    let needing = text
        .replace("---@type ProductStore", "---@type MissingStore")
        .replace("product_store = nil", "missing_store = nil");
    std::fs::write(&controller, &needing).expect("controller");
    let output = Command::new(env!("CARGO_BIN_EXE_ty"))
        .args([
            "handler",
            "invoke",
            "server/controllers/ProductController.lua",
        ])
        .args(["--diagnostic-format", "json", "--project"])
        .arg(root.path())
        .output()
        .expect("ty handler invoke");
    assert!(!output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("diagnostic report JSON");
    let diagnostic = &report["diagnostics"][0];
    assert_eq!(diagnostic["code"], "TY2015", "{report}");
    let span = &diagnostic["spans"][0];
    let start = usize::try_from(span["start"].as_u64().expect("start")).expect("start");
    let end = usize::try_from(span["end"].as_u64().expect("end")).expect("end");
    assert_eq!(&needing[start..end], "  missing_store = nil,", "{report}");
}

#[test]
fn a_lua_converter_writes_null_for_a_missing_source() {
    if !LuaYonProject::lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = LuaYonProject::project("lua-yon-converter");
    let output = LuaYonProject::invoke(
        root.path(),
        "server/controllers/CatalogController.lua",
        "/catalog",
    );
    assert_eq!(
        LuaYonProject::body(&output),
        serde_json::json!({
            "summary": {"productId": 7, "title": "Anvil", "absent": null, "kind": "product"},
        })
    );
}

#[test]
fn a_lua_schedule_marker_runs_the_worker_under_ty_serve() {
    if !LuaYonProject::lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let marker = root.path().join("beats.txt");
    let quoted = format!("{:?}", marker.to_string_lossy());
    let worker = LuaYonProject::WORKER.replace("MARKER", &quoted);
    LuaYonProject::write(root.path(), "server/workers/beat.lua", &worker);
    for (relative, text) in [LuaYonProject::PAGE, LuaYonProject::COMPONENT] {
        LuaYonProject::write(root.path(), relative, text);
    }
    let mut server = Command::new(env!("CARGO_BIN_EXE_ty"))
        .arg("serve")
        .arg(root.path())
        .args(["--port", "0", "--no-self-restart"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("ty serve");
    let started = Instant::now();
    while LuaYonProject::beats(&marker) < 2 && started.elapsed() < LuaYonProject::WORKER_WAIT {
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = server.kill();
    let _ = server.wait();
    assert!(
        LuaYonProject::beats(&marker) >= 2,
        "the scheduled Lua worker did not run twice"
    );
}
