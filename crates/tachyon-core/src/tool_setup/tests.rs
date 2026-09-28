#![allow(clippy::expect_used)]

use super::{
    Consent, HostPlatform, InstallContext, InstallPlan, InstallRecipes, InstallStep, NeededTool,
    PlanKind, Sha256Check, ToolCommand, ToolRunner, ToolSetup,
};
use crate::Failure;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

struct FakeRunner {
    commands: Vec<ToolCommand>,
    archive: Vec<u8>,
    published_hash: String,
}

impl FakeRunner {
    fn new(archive: &[u8], published_hash: &str) -> Self {
        Self {
            commands: Vec::new(),
            archive: archive.to_vec(),
            published_hash: String::from(published_hash),
        }
    }

    fn texts(&self) -> Vec<String> {
        self.commands.iter().map(ToolCommand::text).collect()
    }

    fn output_file(command: &ToolCommand) -> Option<PathBuf> {
        let position = command
            .arguments
            .iter()
            .position(|argument| argument == "--output")?;
        command.arguments.get(position + 1).map(PathBuf::from)
    }
}

impl ToolRunner for FakeRunner {
    fn run(&mut self, command: &ToolCommand) -> Result<(), Failure> {
        self.commands.push(command.clone());
        if let Some(file) = Self::output_file(command) {
            let name = file
                .file_name()
                .expect("name")
                .to_string_lossy()
                .into_owned();
            let contents = if name.ends_with(".sha256-list") {
                format!(
                    "{}  {}\n",
                    self.published_hash,
                    name.trim_end_matches(".sha256-list")
                )
                .into_bytes()
            } else {
                self.archive.clone()
            };
            std::fs::write(file, contents).expect("fake download");
        }
        Ok(())
    }
}

fn context(home: &Path, system: &str) -> InstallContext {
    InstallContext {
        platform: HostPlatform::of(system, true),
        home: home.to_path_buf(),
        project: home.join("app"),
    }
}

fn hash(bytes: &[u8]) -> String {
    crate::compiler::hex_digest(Sha256::digest(bytes))
}

fn run_setup(
    plans: &[InstallPlan],
    runner: &mut FakeRunner,
    consent: Consent,
    answers: &str,
) -> (Vec<NeededTool>, String) {
    let mut input = answers.as_bytes();
    let mut output = Vec::new();
    let missing = ToolSetup::new(&mut input, &mut output, runner, consent)
        .run(plans)
        .expect("setup");
    (missing, String::from_utf8(output).expect("text"))
}

#[test]
fn a_go_install_downloads_verifies_and_unpacks_into_the_home_folder() {
    let home = tempfile::tempdir().expect("home");
    let plan = InstallRecipes::plan(NeededTool::Go, &context(home.path(), HostPlatform::MACOS));
    assert_eq!(plan.kind, PlanKind::Automatic);
    let archive = b"go archive";
    let mut runner = FakeRunner::new(archive, &hash(archive));
    let (missing, output) = run_setup(&[plan], &mut runner, Consent::Always, "");
    assert!(missing.is_empty(), "{output}");
    let commands = runner.texts();
    assert!(
        commands[0].contains("https://go.dev/dl/go1.27.1.darwin-arm64.tar.gz"),
        "{commands:?}"
    );
    assert!(
        commands[1].contains("https://dl.google.com/go/go1.27.1.darwin-arm64.tar.gz.sha256"),
        "{commands:?}"
    );
    assert!(commands[2].starts_with("tar -xzf"), "{commands:?}");
    assert!(
        commands[2].contains(
            &home
                .path()
                .join("sdk/go1.27.1")
                .to_string_lossy()
                .into_owned()
        )
    );
    assert!(output.contains("check the SHA-256"), "{output}");
    assert!(output.contains("Go is installed."), "{output}");
    assert!(commands.iter().all(|command| !command.starts_with("sudo")));
}

#[test]
fn a_wrong_published_hash_stops_before_unpacking_and_deletes_the_download() {
    let home = tempfile::tempdir().expect("home");
    let plan = InstallRecipes::plan(NeededTool::Dart, &context(home.path(), HostPlatform::LINUX));
    let mut runner = FakeRunner::new(b"dart archive", &"0".repeat(64));
    let mut input = "".as_bytes();
    let mut output = Vec::new();
    let error = ToolSetup::new(&mut input, &mut output, &mut runner, Consent::Always)
        .run(&[plan])
        .expect_err("hash mismatch");
    assert!(error.to_string().contains("TY1409"), "{error}");
    assert!(error.to_string().contains("cannot be trusted"), "{error}");
    assert_eq!(runner.commands.len(), 2, "no unzip after a failed check");
    assert!(
        !home
            .path()
            .join(".tachyon/downloads/dartsdk-linux-arm64-release.zip")
            .exists()
    );
}

#[test]
fn each_tool_is_asked_about_and_a_no_keeps_it_missing() {
    let home = tempfile::tempdir().expect("home");
    let context = context(home.path(), HostPlatform::MACOS);
    let plans = InstallRecipes::plans(&[NeededTool::WasmTarget, NeededTool::WasmTools], &context);
    let mut runner = FakeRunner::new(b"", "");
    let (missing, output) = run_setup(&plans, &mut runner, Consent::Ask, "y\nn\n");
    assert_eq!(missing, vec![NeededTool::WasmTools]);
    assert_eq!(output.matches("now? Type y for yes").count(), 2, "{output}");
    let commands = runner.texts();
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert!(
        commands[0].ends_with("target add wasm32-unknown-unknown"),
        "{commands:?}"
    );
}

#[test]
fn without_a_terminal_or_yes_nothing_runs_and_the_commands_are_printed() {
    let home = tempfile::tempdir().expect("home");
    let plan = InstallRecipes::plan(NeededTool::Bun, &context(home.path(), HostPlatform::MACOS));
    let mut runner = FakeRunner::new(b"", "");
    let (missing, output) = run_setup(&[plan], &mut runner, Consent::Never, "");
    assert_eq!(missing, vec![NeededTool::Bun]);
    assert!(runner.commands.is_empty());
    assert!(
        output.contains("bun-v1.3.14/bun-darwin-aarch64.zip"),
        "{output}"
    );
    assert!(output.contains("--yes"), "{output}");
}

#[test]
fn admin_and_manual_tools_are_only_explained() {
    let home = tempfile::tempdir().expect("home");
    let linux = context(home.path(), HostPlatform::LINUX);
    let windows = context(home.path(), HostPlatform::WINDOWS);
    let plans = vec![
        InstallRecipes::plan(NeededTool::Php, &linux),
        InstallRecipes::plan(NeededTool::AndroidSdk, &windows),
        InstallRecipes::plan(NeededTool::Xcode, &linux),
    ];
    assert_eq!(plans[0].kind, PlanKind::NeedsAdmin);
    assert_eq!(plans[1].kind, PlanKind::Manual);
    let mut runner = FakeRunner::new(b"", "");
    let (missing, output) = run_setup(&plans, &mut runner, Consent::Always, "");
    assert_eq!(missing.len(), 3);
    assert!(runner.commands.is_empty(), "sudo is never run");
    assert!(
        output.contains("sudo apt-get install -y php-cgi"),
        "{output}"
    );
    assert!(
        output.contains("https://developer.android.com/studio"),
        "{output}"
    );
    assert!(output.contains("only on a Mac"), "{output}");
}

#[test]
fn every_tool_has_a_plan_on_every_system_and_automatic_plans_stay_in_the_home_folder() {
    let home = tempfile::tempdir().expect("home");
    for system in [
        HostPlatform::MACOS,
        HostPlatform::LINUX,
        HostPlatform::WINDOWS,
    ] {
        let context = context(home.path(), system);
        for tool in NeededTool::ALL {
            let plan = InstallRecipes::plan(tool, &context);
            assert_eq!(plan.tool, tool);
            assert!(
                !plan.lines.is_empty() || !plan.steps.is_empty(),
                "{tool:?} on {system}"
            );
            for step in &plan.steps {
                let text = step.text();
                assert!(!text.contains("sudo"), "{tool:?} on {system}: {text}");
                if let InstallStep::CreateFolder(folder)
                | InstallStep::MoveFolder { to: folder, .. } = step
                {
                    assert!(
                        folder.starts_with(home.path()),
                        "{tool:?} on {system}: {text}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_emscripten_plan_uses_the_pinned_emsdk_hash() {
    let home = tempfile::tempdir().expect("home");
    let plan = InstallRecipes::plan(
        NeededTool::Emscripten,
        &context(home.path(), HostPlatform::MACOS),
    );
    let pinned = plan.steps.iter().any(|step| {
        matches!(step, InstallStep::CheckPinned { sha256, .. } if sha256 == "8634b12869818ff088757141804dd4b36713add0a5669a14de1a7f72609c26df")
    });
    assert!(pinned, "{:?}", plan.steps);
    assert!(
        plan.steps
            .iter()
            .any(|step| step.text().contains("emsdk.py install 6.0.10"))
    );
}

#[test]
fn listed_hashes_are_found_by_file_name_or_first_line() {
    let list = "aaa  bun-darwin-aarch64-profile.zip\nbbb  bun-darwin-aarch64.zip\n";
    assert_eq!(
        Sha256Check::listed_hash(list, "bun-darwin-aarch64.zip"),
        "bbb"
    );
    assert_eq!(
        Sha256Check::listed_hash("CCC *./rustup-init\n", "rustup-init"),
        "ccc"
    );
    assert_eq!(Sha256Check::listed_hash("ddd\n", "go.tar.gz"), "ddd");
    assert_eq!(Sha256Check::listed_hash("", "go.tar.gz"), "");
}

#[test]
fn a_command_text_quotes_arguments_with_spaces() {
    let command = ToolCommand::new("bash", &["-c", "sdk install java 21"]);
    assert_eq!(command.text(), "bash -c \"sdk install java 21\"");
    assert_eq!(Consent::from_options(true, false), Consent::Always);
    assert_eq!(Consent::from_options(false, true), Consent::Ask);
    assert_eq!(Consent::from_options(false, false), Consent::Never);
    assert_eq!(NeededTool::for_command("php-cgi"), Some(NeededTool::Php));
    assert_eq!(
        NeededTool::for_command("rustc --target wasm32-unknown-unknown"),
        Some(NeededTool::WasmTarget)
    );
    assert_eq!(NeededTool::for_command("fylo"), None);
}
