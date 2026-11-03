#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct ToolTrap {
    folder: PathBuf,
    mark: PathBuf,
}

impl ToolTrap {
    const TOOLS: [&'static str; 6] = ["dart", "cargo", "go", "dotnet", "bun", "node"];

    fn new(workspace: &Path) -> Self {
        let folder = workspace.join("tools");
        let mark = workspace.join("tool-started");
        fs::create_dir_all(&folder).expect("tool folder");
        for tool in Self::TOOLS {
            let script = format!("#!/bin/sh\necho {tool} >> '{}'\nexit 1\n", mark.display());
            Self::write_program(&folder.join(tool), &script);
        }
        Self { folder, mark }
    }

    fn write_program(path: &Path, script: &str) {
        use std::os::unix::fs::PermissionsExt as _;
        fs::write(path, script).expect("tool script");
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("executable");
    }

    fn bundle(&self, project: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ty"))
            .arg("bundle")
            .arg(project)
            .env("PATH", format!("{}:/usr/bin:/bin", self.folder.display()))
            .output()
            .expect("ty bundle")
    }

    fn started(&self) -> String {
        fs::read_to_string(&self.mark).unwrap_or_default()
    }
}

fn initialized(workspace: &Path, name: &str) -> PathBuf {
    let project = workspace.join(name);
    let output = Command::new(env!("CARGO_BIN_EXE_ty"))
        .arg("init")
        .arg(&project)
        .args(["--realm", "client", "--tac", "js,dart", "--skip-tools"])
        .output()
        .expect("ty init");
    assert!(output.status.success(), "{}", text(&output));
    let service = project.join("client/services/DartGreetingService.dart");
    let unique = workspace
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let source = fs::read_to_string(&service).expect("Dart service");
    fs::write(&service, source.replace("Hello from Dart", unique)).expect("unique Dart service");
    project
}

fn write(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    fs::write(path, source).expect("source");
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn a_valid_project_reaches_the_dart_toolchain_so_the_trap_is_real() {
    let workspace = tempfile::tempdir().expect("workspace");
    let trap = ToolTrap::new(workspace.path());
    let project = initialized(workspace.path(), "valid");
    let output = trap.bundle(&project);
    assert!(trap.started().contains("dart"), "{}", text(&output));
}

#[test]
fn an_invalid_structure_fails_before_any_output_or_toolchain_and_lists_every_problem() {
    let workspace = tempfile::tempdir().expect("workspace");
    let trap = ToolTrap::new(workspace.path());
    let project = initialized(workspace.path(), "invalid");
    write(
        &project,
        "client/services/billing/BillingThing.js",
        "export default class BillingThing {\n  _total = 0\n}\n",
    );
    write(
        &project,
        "client/services/link/LinkService.dart",
        "import '../../../server/controllers/HomeController.dart';\n@Service\nclass LinkService {\n  String label = 'x';\n}\n",
    );
    write(
        &project,
        "client/stores/Cart.js",
        "@Store\nexport default class Cart {\n  _total = 0\n}\n",
    );
    let output = trap.bundle(&project);
    let shown = text(&output);
    assert!(!output.status.success(), "{shown}");
    for (code, file) in [
        ("TY1363", "client/services/billing/BillingThing.js"),
        ("TY1369", "client/services/link/LinkService.dart"),
        ("TY1310", "client/stores/Cart.js"),
    ] {
        assert!(shown.contains(code), "{code}: {shown}");
        assert!(shown.contains(file), "{file}: {shown}");
    }
    assert_eq!(trap.started(), "", "{shown}");
    assert!(!project.join("dist").exists(), "{shown}");
}
