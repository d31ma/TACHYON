#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

struct ItemsFixture;

impl ItemsFixture {
    const CONTROLLER: &str = "server/controllers/ItemsController.js";

    const BOTH_ROUTES: &str = r"@RequestRoute('/items')
@Public
export class ItemsController {
  /** @param {number} id */
  @GetRoute('/{id}')
  get(@FromPath id) {
    return { id };
  }

  /** @param {string} name */
  @PostRoute('/echo')
  postEcho(@FromQuery name) {
    return { name };
  }
}
";

    const ONE_ROUTE: &str = r"@RequestRoute('/items')
@Public
export class ItemsController {
  /** @param {number} id */
  @GetRoute('/{id}')
  get(@FromPath id) {
    return { id };
  }
}
";
}

fn ty() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ty"))
}

fn run(command: &mut Command) -> Output {
    command.output().expect("the ty process should start")
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
    fs::write(path, contents).expect("fixture source");
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn initialized(workspace: &Path, name: &str) -> std::path::PathBuf {
    let project = workspace.join(name);
    let init = run(ty().arg("init").arg(&project).arg("--skip-tools"));
    assert!(init.status.success(), "{}", stderr(&init));
    project
}

#[test]
fn api_freeze_saves_the_baseline_that_later_bundles_enforce() {
    let workspace = tempfile::tempdir().expect("workspace");
    let project = initialized(workspace.path(), "frozen");
    write(
        &project.join(ItemsFixture::CONTROLLER),
        ItemsFixture::BOTH_ROUTES,
    );

    let frozen = run(ty().args(["api", "freeze"]).arg(&project));
    assert!(frozen.status.success(), "{}", stderr(&frozen));
    assert!(stdout(&frozen).contains("Wrote the API baseline"));
    let baseline =
        fs::read_to_string(project.join("server/api-baseline.json")).expect("baseline file");
    assert!(baseline.contains("/items/echo"), "{baseline}");
    assert!(baseline.contains("application_version"), "{baseline}");

    let unchanged = run(ty().arg("bundle").arg(&project));
    assert!(unchanged.status.success(), "{}", stderr(&unchanged));

    write(
        &project.join(ItemsFixture::CONTROLLER),
        ItemsFixture::ONE_ROUTE,
    );
    let broken = run(ty().arg("bundle").arg(&project));
    assert!(!broken.status.success());
    let message = stderr(&broken);
    assert!(message.contains("TY1338"), "{message}");
    assert!(message.contains("/items/echo"), "{message}");
}

#[test]
fn cache_status_and_clean_work_on_the_tac_cache_dir() {
    let cache = tempfile::tempdir().expect("cache");
    write(&cache.path().join("runtime/native-v1/entry"), "runtime");
    write(&cache.path().join("go-wasm/build/output"), "build");
    write(&cache.path().join("kept/file"), "not ours");

    let status = run(ty().arg("cache").env("TAC_CACHE_DIR", cache.path()));
    assert!(status.status.success(), "{}", stderr(&status));
    let report = stdout(&status);
    assert!(report.contains("Runtime entries: 1"), "{report}");
    assert!(
        report.contains("Build cache go-wasm: 1 entries"),
        "{report}"
    );

    let cleaned = run(ty()
        .args(["cache", "clean"])
        .env("TAC_CACHE_DIR", cache.path()));
    assert!(cleaned.status.success(), "{}", stderr(&cleaned));
    assert!(stdout(&cleaned).contains("Cleared Tachyon runtime and build caches"));
    assert!(!cache.path().join("runtime").exists());
    assert!(!cache.path().join("go-wasm").exists());
    assert!(cache.path().join("kept/file").is_file());
}

#[test]
fn bundle_explain_targets_reports_support_without_building() {
    let workspace = tempfile::tempdir().expect("workspace");
    let project = initialized(workspace.path(), "explained");

    let explained =
        run(ty()
            .arg("bundle")
            .arg(&project)
            .args(["--explain-targets", "--target", "web"]));
    assert!(explained.status.success(), "{}", stderr(&explained));
    let report = stdout(&explained);
    assert!(report.starts_with("Web\n"), "{report}");
    for other in ["macOS", "Windows", "Linux", "iOS", "Android"] {
        let heading = format!("\n{other}\n\n");
        assert!(!report.contains(&heading), "{other}: {report}");
    }
    assert!(!project.join("dist").exists());
}

#[test]
fn servers_refuse_a_non_loopback_host_without_explicit_permission() {
    let workspace = tempfile::tempdir().expect("workspace");
    let project = initialized(workspace.path(), "exposed");

    for command in ["serve", "start", "dev", "preview"] {
        let refused = run(ty()
            .arg(command)
            .arg(&project)
            .args(["--host", "0.0.0.0", "--port", "0"]));
        assert!(!refused.status.success(), "{command}");
        let message = stderr(&refused);
        assert!(message.contains("TY1301"), "{command}: {message}");
        assert!(
            message.contains("without explicit permission"),
            "{command}: {message}"
        );
    }

    let permitted = run(ty().arg("start").arg(&project).args([
        "--host",
        "0.0.0.0",
        "--port",
        "0",
        "--allow-non-loopback",
        "--out-dir",
        "missing-bundle",
    ]));
    assert!(!permitted.status.success());
    let message = stderr(&permitted);
    assert!(!message.contains("TY1301"), "{message}");
    assert!(message.contains("TY1304"), "{message}");
}

#[test]
fn doctor_install_flags_are_checked_before_anything_runs() {
    for arguments in [
        &["doctor", "--yes"][..],
        &["doctor", "--install", "--json"][..],
    ] {
        let refused = run(ty().args(arguments));
        assert_eq!(refused.status.code(), Some(2), "{arguments:?}");
    }
}

#[test]
fn compatibility_flags_are_still_accepted() {
    for arguments in [
        &["dev", "--bundle-watch", "--help"][..],
        &["preview", "--skip-native-checks", "--help"][..],
    ] {
        let accepted = run(ty().args(arguments));
        assert!(
            accepted.status.success(),
            "{arguments:?}: {}",
            stderr(&accepted)
        );
    }
}

#[test]
fn doctor_migrate_and_api_help_describe_what_they_do() {
    let help = run(ty().arg("help"));
    assert!(help.status.success(), "{}", stderr(&help));
    let help = stdout(&help);
    for command in ["doctor", "migrate", "api"] {
        assert!(help.contains(command), "missing {command}: {help}");
    }

    let migrate = run(ty().args(["migrate", "--help"]));
    assert!(migrate.status.success(), "{}", stderr(&migrate));
    assert!(stdout(&migrate).contains("ty migrate fix --write"));

    let api = run(ty().args(["api", "--help"]));
    assert!(api.status.success(), "{}", stderr(&api));
    let api = stdout(&api);
    assert!(api.contains("baseline"), "{api}");
    assert!(!api.contains("or check"), "{api}");
}

#[test]
fn handler_invoke_refuses_to_pass_ty_variables() {
    let workspace = tempfile::tempdir().expect("workspace");
    write(
        &workspace.path().join(ItemsFixture::CONTROLLER),
        ItemsFixture::ONE_ROUTE,
    );

    for name in ["TAC_TOKEN", "YON_SECRET", "HTTPS_PROXY", "tachyon_home"] {
        let refused = run(ty()
            .args(["handler", "invoke", ItemsFixture::CONTROLLER])
            .arg("--project")
            .arg(workspace.path())
            .args(["--allow-env", name]));
        assert!(!refused.status.success(), "{name}");
        let message = stderr(&refused);
        assert!(message.contains("TY2006"), "{name}: {message}");
        assert!(message.contains("ty's own variables"), "{name}: {message}");
    }
}
