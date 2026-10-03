#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn ty() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ty"))
}

fn run(command: &mut Command) -> Output {
    command.output().expect("the ty process should start")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
    fs::write(path, contents).expect("fixture source");
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root")
        .to_path_buf()
}

#[test]
fn a_supported_project_passes_the_migration_check() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view1 />",
    );
    write(
        &project.path().join("client/components/test/view1/tac.html"),
        "<main aria-label=\"Home\"><h1>Home</h1></main>",
    );
    let output = run(ty().arg("migrate").arg("check").arg(project.path()));
    assert!(output.status.success(), "{}", stdout(&output));
    assert!(stdout(&output).contains("0 unsupported"));
}

#[test]
fn unsupported_constructs_fail_closed_with_a_stable_code() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view2 />",
    );
    write(
        &project.path().join("client/components/test/view2/tac.html"),
        "<main>x</main>",
    );
    write(&project.path().join("client/pages/tac.py"), "x = 1");

    let strict = run(ty().arg("migrate").arg("check").arg(project.path()));
    assert!(!strict.status.success());
    assert!(String::from_utf8_lossy(&strict.stderr).contains("TY1702"));

    let permissive = run(ty()
        .arg("migrate")
        .arg("check")
        .arg(project.path())
        .arg("--allow-unsupported"));
    assert!(permissive.status.success());
    assert!(stdout(&permissive).contains("companion.polyglot"));
}

#[test]
fn the_report_is_deterministic_and_schema_shaped() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view3 />",
    );
    write(
        &project.path().join("client/components/test/view3/tac.html"),
        "<main>x</main>",
    );
    write(
        &project.path().join("client/pages/tac.js"),
        "export default {}",
    );
    write(
        &project.path().join("server/routes/yon.py"),
        "class Handler: pass",
    );

    let first = run(ty()
        .arg("migrate")
        .arg("check")
        .arg(project.path())
        .args(["--json", "--allow-unsupported"]));
    let second = run(ty()
        .arg("migrate")
        .arg("check")
        .arg(project.path())
        .args(["--json", "--allow-unsupported"]));
    assert!(first.status.success());
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "report is not deterministic"
    );

    let report: serde_json::Value =
        serde_json::from_str(&stdout(&first)).expect("Migration Report v1 JSON");
    assert_migration_report_shape(&report);
}

fn assert_migration_report_shape(report: &serde_json::Value) {
    assert_eq!(report["contract_version"], 1);
    let findings = report["findings"].as_array().expect("findings");
    assert!(!findings.is_empty());
    for finding in findings {
        assert!(finding["source"].is_string());
        assert!(finding["feature"].is_string());
        assert!(finding["detail"].is_string());
        let status = finding["status"].as_str().expect("status");
        assert!(matches!(status, "supported" | "changed" | "unsupported"));
        if status != "supported" {
            assert!(
                finding["action"].is_string(),
                "{} carries no action",
                finding["feature"]
            );
        }
    }
}

#[test]
fn the_migration_fixture_is_classified_without_being_executed() {
    let fixture = repository_root().join("crates/tachyon-cli/tests/fixtures/migration-fullstack");
    assert!(fixture.is_dir(), "the migration fixture is missing");
    let output = run(ty()
        .arg("migrate")
        .arg("check")
        .arg(&fixture)
        .args(["--json", "--allow-unsupported"]));
    assert!(output.status.success(), "{}", stdout(&output));

    let report: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("report");
    let features: Vec<&str> = report["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter_map(|finding| finding["feature"].as_str())
        .collect();
    for expected in [
        "handler.supervised",
        "handler.other_language",
        "handler.dependency",
        "server.worker",
    ] {
        assert!(features.contains(&expected), "missing {expected}");
    }
    assert!(
        report["unsupported"].as_u64().unwrap_or_default() > 0,
        "the migration fixture should not be fully supported yet"
    );
}

#[test]
fn every_corpus_project_builds_under_this_implementation() {
    let corpus = repository_root().join("corpus");
    if !corpus.is_dir() {
        return;
    }
    let mut projects: Vec<PathBuf> = fs::read_dir(&corpus)
        .expect("corpus")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .collect();
    projects.sort();
    assert!(!projects.is_empty(), "the corpus is empty");

    for project in projects {
        let staged = tempfile::tempdir().expect("staging");
        copy_tree(&project, staged.path());
        assert_corpus_project_builds(&project, staged.path());
    }
}

fn assert_corpus_project_builds(project: &Path, staged: &Path) {
    let build = run(ty()
        .arg("build")
        .arg(staged)
        .args(["--out-dir", "dist-rust"]));
    assert!(
        build.status.success(),
        "{} failed: {}",
        project.display(),
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(staged.join("dist-rust/route-manifest.json").is_file());

    let check = run(ty().arg("migrate").arg("check").arg(staged));
    assert!(
        check.status.success(),
        "{} has unsupported constructs: {}",
        project.display(),
        stdout(&check)
    );
}

fn copy_tree(source: &Path, destination: &Path) {
    for entry in fs::read_dir(source).expect("corpus entries") {
        let entry = entry.expect("corpus entry");
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            fs::create_dir_all(&target).expect("directory");
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("file");
        }
    }
}

fn write_project_before_the_breaking_changes(root: &Path) {
    write(&root.join("VERSION"), "26.36.01\n");
    write(
        &root.join("scripts/build.js"),
        "import { Service } from 'tachyon:behaviors'\n",
    );
    write(&root.join("client/pages/tac.html"), "<test-view9 />");
    write(
        &root.join("client/components/test/view9/tac.html"),
        "<main aria-label=\"Home\"><h1>Home</h1></main>",
    );
    write(
        &root.join("client/services/CartService.js"),
        "import { Service } from 'tachyon:behaviors'\n\n@Service\nexport default class CartService {\n  @Autowired\n  cartComponent\n\n  @PostConstruct\n  start() {\n    this.cartComponent.items.push(1)\n  }\n\n  total() {\n    return this.cartComponent.items.length\n  }\n}\n",
    );
    write(
        &root.join("client/delegates/CartComponent.js"),
        "@Component\nexport default class CartComponent {\n  items = []\n}\n",
    );
    write(
        &root.join("server/routes/yon.js"),
        "export class Handler {\n  static GET() {\n    return { ok: true }\n  }\n}\n",
    );
    write(
        &root.join("middleware.js"),
        "export class Handler {\n  static GET(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n\n  static POST(request) {\n    return { status: 204, headers: {}, body: '' }\n  }\n}\n",
    );
}

fn build(project: &Path) -> Output {
    run(ty()
        .arg("build")
        .arg(project)
        .args(["--out-dir", "dist-rust"]))
}

#[test]
fn migrate_fix_writes_the_automatic_fixes_and_the_project_builds() {
    let project = tempfile::tempdir().expect("project");
    write_project_before_the_breaking_changes(project.path());
    let before = build(project.path());
    assert!(!before.status.success(), "the old project should not build");

    let preview = run(ty().arg("migrate").arg("fix").arg(project.path()));
    assert!(preview.status.success(), "{}", stdout(&preview));
    assert!(
        stdout(&preview).contains("4 file(s) to change, 2 file(s) to move"),
        "{}",
        stdout(&preview)
    );
    assert!(
        !build(project.path()).status.success(),
        "a preview must not write"
    );

    let written = run(ty()
        .arg("migrate")
        .arg("fix")
        .arg(project.path())
        .arg("--write"));
    assert!(written.status.success(), "{}", stdout(&written));
    assert!(
        stdout(&written).contains("changed  middleware.js"),
        "{}",
        stdout(&written)
    );
    assert!(
        stdout(&written)
            .contains("moved    client/delegates/CartComponent.js -> client/helpers/CartHelper.js"),
        "{}",
        stdout(&written)
    );
    let after = build(project.path());
    assert!(
        after.status.success(),
        "{}",
        String::from_utf8_lossy(&after.stderr)
    );

    assert_eq!(
        fs::read_to_string(project.path().join("VERSION")).expect("VERSION"),
        "26.36.01\n"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("scripts/build.js")).expect("script"),
        "import { Service } from 'tachyon:behaviors'\n"
    );

    let again = run(ty()
        .arg("migrate")
        .arg("fix")
        .arg(project.path())
        .arg("--write"));
    assert!(
        stdout(&again).contains("0 file(s) changed, 0 manual step(s)"),
        "{}",
        stdout(&again)
    );
}
