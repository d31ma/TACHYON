#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn ty() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ty"))
}

fn run(command: &mut Command) -> Output {
    command.output().expect("the command should start")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn write(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, source).expect("source");
}

fn composed(root: &Path, realm: &str) -> String {
    fs::read_to_string(
        root.join(".tachyon/types")
            .join(realm)
            .join("composed.d.ts"),
    )
    .expect("generated types")
}

fn typescript_compiler() -> Option<PathBuf> {
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../node_modules/.bin/tsc");
    let program = if local.is_file() {
        local
    } else {
        PathBuf::from("tsc")
    };
    let version = Command::new(&program).arg("--version").output().ok()?;
    version.status.success().then_some(program)
}

fn type_check(compiler: &Path, project: &Path, realm: &str) -> Output {
    let config = project.join(realm).join("jsconfig.json");
    run(Command::new(compiler).arg("-p").arg(config).arg("--noEmit"))
}

#[test]
fn the_types_command_writes_each_realm_and_drops_deleted_classes() {
    let project = tempfile::tempdir().expect("project");
    write(
        project.path(),
        "client/services/ClockService.js",
        "@Service\nexport default class ClockService {\n  now() { return 0 }\n}\n",
    );
    let output = run(ty().arg("types").arg(project.path()));
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains(".tachyon/types/client/composed.d.ts"),
        "{}",
        text(&output.stdout)
    );
    let client = composed(project.path(), "client");
    assert!(
        client.contains(
            "  const ClockService: typeof import(\"../../../client/services/ClockService.js\").default;\n"
        ),
        "{client}"
    );
    assert_eq!(composed(project.path(), "server"), "export {};\n");

    fs::remove_file(project.path().join("client/services/ClockService.js")).expect("delete");
    let output = run(ty().arg("types").arg(project.path()));
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(composed(project.path(), "client"), "export {};\n");
}

#[test]
fn a_role_type_is_declared_for_the_routes_that_allow_it() {
    let project = tempfile::tempdir().expect("project");
    write(
        project.path(),
        "server/roles/Admin.ts",
        "export default class Admin {}\n",
    );
    let output = run(ty().arg("types").arg(project.path()));
    assert!(output.status.success(), "{}", text(&output.stderr));
    let server = composed(project.path(), "server");
    assert!(
        server.contains(
            "  const Admin: typeof import(\"../../../server/roles/Admin.ts\").default;\n"
        ),
        "{server}"
    );
}

#[test]
fn the_types_command_reports_a_missing_project() {
    let workspace = tempfile::tempdir().expect("workspace");
    let output = run(ty().arg("types").arg(workspace.path().join("missing")));
    assert!(!output.status.success());
    assert!(
        text(&output.stderr).contains("TY1410"),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn a_scaffolded_project_uses_its_service_without_an_import() {
    let workspace = tempfile::tempdir().expect("workspace");
    let project = workspace.path().join("app");
    let output = run(ty().arg("init").arg(&project));
    assert!(output.status.success(), "{}", text(&output.stderr));
    let test = fs::read_to_string(project.join("client/tests/ShellServiceTest.js")).expect("test");
    assert!(
        test.contains("this.construct(ShellService)") && !test.contains("import"),
        "{test}"
    );
    assert!(test.contains("@Fake\n  documentTitle"), "{test}");
    assert!(!test.contains("import"), "{test}");
    assert!(composed(&project, "client").contains("const ShellService: typeof import("));

    let Some(compiler) = typescript_compiler() else {
        eprintln!("skipping the type check: the TypeScript compiler is not installed");
        return;
    };
    for realm in ["client", "server"] {
        let checked = type_check(&compiler, &project, realm);
        assert!(
            checked.status.success(),
            "{realm}: {}",
            text(&checked.stdout)
        );
    }

    write(
        &project,
        "server/services/ProbeService.js",
        "@Service\nexport default class ProbeService {\n  shell() { return new ShellService() }\n}\n",
    );
    let checked = type_check(&compiler, &project, "server");
    assert!(!checked.status.success());
    assert!(
        text(&checked.stdout).contains("Cannot find name 'ShellService'"),
        "{}",
        text(&checked.stdout)
    );
}

#[test]
fn injected_javascript_fields_pass_the_type_checker_and_the_tests() {
    let workspace = tempfile::tempdir().expect("workspace");
    let project = workspace.path().join("app");
    let output = run(ty().arg("init").arg(&project).arg("--skip-tools"));
    assert!(output.status.success(), "{}", text(&output.stderr));
    let tested = run(ty().args(["test", "--coverage"]).arg(&project));
    assert!(tested.status.success(), "{}", text(&tested.stdout));

    write(
        &project,
        "server/services/PriceService.js",
        "@Service\nexport default class PriceService {\n  /** @returns {number} */\n  price() { return 3 }\n}\n",
    );
    write(
        &project,
        "server/services/CartService.js",
        "@Service\nexport default class CartService {\n  /** @type {PriceService} */\n  @Needs\n  priceService\n\n  /** @returns {number} */\n  total() { return this.priceService.price() * 2 }\n}\n",
    );
    let output = run(ty().arg("types").arg(&project));
    assert!(output.status.success(), "{}", text(&output.stderr));
    let Some(compiler) = typescript_compiler() else {
        eprintln!("skipping the type check: the TypeScript compiler is not installed");
        return;
    };
    let checked = type_check(&compiler, &project, "server");
    assert!(checked.status.success(), "{}", text(&checked.stdout));
}
