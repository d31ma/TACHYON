#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct FeatureProject {
    name: &'static str,
    languages: &'static [&'static str],
}

impl FeatureProject {
    const EVERY_FEATURE: &'static str = "storage,sockets,topics,streams,http-client,error-handlers";
    const REALMS: [&'static str; 2] = ["client", "server"];
    const RUNTIMES: [&'static str; 2] = ["node", "bun"];
    const CASES: [Self; 2] = [
        Self {
            name: "javascript",
            languages: &[],
        },
        Self {
            name: "typescript",
            languages: &["--tac", "ts", "--yon", "ts"],
        },
    ];

    fn initialized(&self, workspace: &Path, modules: &Path) -> PathBuf {
        let project = workspace.join(self.name);
        let output = run(ty().arg("init").arg(&project).args(self.languages).args([
            "--features",
            Self::EVERY_FEATURE,
            "--skip-tools",
        ]));
        assert!(output.status.success(), "{}: {}", self.name, text(&output));
        link_modules(&project, modules);
        project
    }

    fn assert_types_check(&self, project: &Path, compiler: &Path) {
        let types = run(ty().arg("types").arg(project));
        assert!(types.status.success(), "{}: {}", self.name, text(&types));
        for realm in Self::REALMS {
            let config = project.join(realm).join("jsconfig.json");
            let checked = run(Command::new(compiler).arg("-p").arg(config));
            assert!(
                checked.status.success(),
                "{} {realm}: {}",
                self.name,
                text(&checked)
            );
        }
    }

    fn assert_tests_pass(&self, project: &Path, runtime: &str) {
        let tested = run(ty()
            .env("YON_JAVASCRIPT_RUNTIME", runtime)
            .args(["test", "--coverage"])
            .arg(project));
        let report = text(&tested);
        assert!(tested.status.success(), "{} {runtime}: {report}", self.name);
        assert!(
            report.contains(" 0 failed."),
            "{} {runtime}: {report}",
            self.name
        );
    }
}

fn ty() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ty"))
}

fn run(command: &mut Command) -> Output {
    command.output().expect("the command should start")
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn repository_modules() -> Option<PathBuf> {
    let modules = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../node_modules");
    modules.join(".bin/tsc").is_file().then_some(modules)
}

fn link_modules(project: &Path, modules: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(modules, project.join("node_modules")).expect("node_modules link");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(modules, project.join("node_modules"))
        .expect("node_modules link");
}

#[test]
fn every_feature_project_type_checks_and_passes_its_tests_on_node_and_bun() {
    let Some(modules) = repository_modules() else {
        eprintln!("skipping: the repository has no TypeScript compiler in node_modules");
        return;
    };
    let runtimes: Vec<&str> = FeatureProject::RUNTIMES
        .into_iter()
        .filter(|runtime| available(runtime))
        .collect();
    let compiler = modules.join(".bin/tsc");
    let workspace = tempfile::tempdir().expect("workspace");
    for case in &FeatureProject::CASES {
        let project = case.initialized(workspace.path(), &modules);
        case.assert_types_check(&project, &compiler);
        for runtime in &runtimes {
            case.assert_tests_pass(&project, runtime);
        }
    }
}
