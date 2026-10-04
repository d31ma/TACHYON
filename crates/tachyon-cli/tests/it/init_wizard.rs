#![allow(clippy::expect_used)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct WizardCase {
    name: &'static str,
    flags: &'static [&'static str],
    programs: &'static [&'static str],
    typescript_client: bool,
}

impl WizardCase {
    const MATRIX: [Self; 7] = [
        Self {
            name: "defaults",
            flags: &[],
            programs: &["bun"],
            typescript_client: false,
        },
        Self {
            name: "client-typescript-dart-rust",
            flags: &[
                "--realm",
                "client",
                "--tac",
                "ts,dart,rs",
                "--target",
                "web,macos",
            ],
            programs: &["bun", "dart", "rustc"],
            typescript_client: true,
        },
        Self {
            name: "server-python-go-java-kotlin",
            flags: &["--realm", "server", "--yon", "py,go,java,kt"],
            programs: &["python3", "go", "javac", "kotlinc"],
            typescript_client: false,
        },
        Self {
            name: "both-csharp-go-ruby-php-rust-dart",
            flags: &["--tac", "cs,go,rb", "--yon", "cs,php,rb,rs,dart"],
            programs: &["dotnet", "go", "ruby", "emcc", "php-cgi", "cargo", "dart"],
            typescript_client: false,
        },
        Self {
            name: "native-swift-kotlin",
            flags: &[
                "--tac",
                "swift,kt",
                "--yon",
                "ts",
                "--target",
                "web,macos,ios,android",
            ],
            programs: &["bun", "swiftc", "kotlinc"],
            typescript_client: false,
        },
        Self {
            name: "javascript-features",
            flags: &[
                "--features",
                "storage,sockets,topics,streams,http-client,error-handlers",
            ],
            programs: &["bun"],
            typescript_client: false,
        },
        Self {
            name: "typescript-features",
            flags: &[
                "--tac",
                "ts",
                "--yon",
                "ts",
                "--features",
                "storage,sockets,topics,streams,http-client,error-handlers",
            ],
            programs: &["bun"],
            typescript_client: true,
        },
    ];

    fn missing_program(&self) -> Option<&'static str> {
        self.programs
            .iter()
            .copied()
            .find(|program| !available(program))
    }
}

fn ty() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
    command.env("YON_JAVASCRIPT_RUNTIME", "bun");
    command
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
        .arg(version_flag(program))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn version_flag(program: &str) -> &'static str {
    match program {
        "kotlinc" => "-version",
        "go" => "version",
        _ => "--version",
    }
}

fn repository_typescript() -> Option<PathBuf> {
    let modules = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../node_modules");
    modules.join(".bin/tsc").is_file().then_some(modules)
}

fn link_typescript(project: &Path, modules: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(modules, project.join("node_modules")).expect("node_modules link");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(modules, project.join("node_modules"))
        .expect("node_modules link");
}

fn initialized(workspace: &Path, case: &WizardCase) -> PathBuf {
    let project = workspace.join(case.name);
    let output = run(ty()
        .arg("init")
        .arg(&project)
        .args(case.flags)
        .arg("--skip-tools"));
    assert!(output.status.success(), "{}: {}", case.name, text(&output));
    project
}

#[test]
fn every_generated_combination_passes_its_tests_with_coverage_and_builds() {
    let workspace = tempfile::tempdir().expect("workspace");
    for case in &WizardCase::MATRIX {
        if let Some(program) = case.missing_program() {
            eprintln!("skipping {}: {program} is unavailable", case.name);
            continue;
        }
        let typescript = repository_typescript();
        if case.typescript_client && typescript.is_none() {
            eprintln!(
                "skipping {}: the repository has no TypeScript compiler in node_modules",
                case.name
            );
            continue;
        }
        let project = initialized(workspace.path(), case);
        if let (true, Some(modules)) = (case.typescript_client, &typescript) {
            link_typescript(&project, modules);
        }
        let tested = run(ty().args(["test", "--coverage"]).arg(&project));
        let report = text(&tested);
        assert!(tested.status.success(), "{}: {report}", case.name);
        assert!(report.contains(" 0 failed."), "{}: {report}", case.name);
        let built = run(ty().arg("build").arg(&project));
        assert!(built.status.success(), "{}: {}", case.name, text(&built));
    }
}

#[test]
fn the_questions_can_be_answered_from_a_script() {
    let workspace = tempfile::tempdir().expect("workspace");
    let project = workspace.path().join("asked");
    let mut child = ty()
        .arg("init")
        .arg(&project)
        .args(["--interactive", "--skip-tools"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ty init");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(b"3\npy\n\n")
        .expect("answers");
    let output = child.wait_with_output().expect("finished");
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    assert!(shown.contains("1. What should the app have?"), "{shown}");
    assert!(
        shown.contains("Which languages do you want for the server (Yon)?"),
        "{shown}"
    );
    assert!(!shown.contains("pages (Tac)? You can"), "{shown}");
    assert!(
        project
            .join("server/controllers/PythonGreetingController.py")
            .is_file()
    );
    assert!(!project.join("client").exists());
}

#[test]
fn without_a_terminal_and_without_options_init_uses_the_defaults_and_only_prints_install_commands()
{
    let workspace = tempfile::tempdir().expect("workspace");
    let project = workspace.path().join("quiet");
    let output = run(ty()
        .arg("init")
        .arg(&project)
        .env("PATH", "")
        .stdin(Stdio::null()));
    let shown = text(&output);
    assert!(output.status.success(), "{shown}");
    assert!(
        shown.contains("Checking the tools that this app needs"),
        "{shown}"
    );
    assert!(
        shown.contains("Bun, the JavaScript and TypeScript runtime is missing"),
        "{shown}"
    );
    assert!(
        shown.contains("Run the command again with --yes"),
        "{shown}"
    );
    assert!(!project.join("node_modules").exists());
    assert!(project.join("client/services/ShellService.js").is_file());
}

#[test]
fn invalid_choices_fail_before_any_file_is_written() {
    let workspace = tempfile::tempdir().expect("workspace");
    let project = workspace.path().join("refused");
    let output =
        run(ty()
            .arg("init")
            .arg(&project)
            .args(["--realm", "client", "--features", "sockets"]));
    assert!(!output.status.success());
    assert!(text(&output).contains("TY1408"), "{}", text(&output));
    assert!(!project.exists());
}

#[test]
fn add_extends_an_existing_app() {
    let workspace = tempfile::tempdir().expect("workspace");
    let project = workspace.path().join("grown");
    assert!(
        run(ty().arg("init").arg(&project).arg("--skip-tools"))
            .status
            .success()
    );
    let language = run(ty()
        .args([
            "add",
            "language",
            "server",
            "rb",
            "--skip-tools",
            "--project",
        ])
        .arg(&project));
    assert!(language.status.success(), "{}", text(&language));
    assert!(text(&language).contains("created server/controllers/RubyGreetingController.rb"));
    let feature = run(ty()
        .args(["add", "feature", "http-client", "--skip-tools", "--project"])
        .arg(&project));
    assert!(feature.status.success(), "{}", text(&feature));
    assert!(project.join("client/remotes/HelloClient.js").is_file());
    let target = run(ty()
        .args(["add", "target", "linux", "--skip-tools", "--project"])
        .arg(&project));
    assert!(target.status.success(), "{}", text(&target));
    let again = run(ty()
        .args(["add", "feature", "http-client", "--skip-tools", "--project"])
        .arg(&project));
    assert!(!again.status.success());
    assert!(text(&again).contains("TY1402"), "{}", text(&again));
    if available("bun") && available("ruby") {
        let tested = run(ty().args(["test", "--coverage"]).arg(&project));
        assert!(tested.status.success(), "{}", text(&tested));
    }
}

#[test]
fn a_java_page_service_passes_its_tests_with_coverage_and_builds_with_teavm() {
    let workspace = tempfile::tempdir().expect("workspace");
    let case = WizardCase {
        name: "client-java",
        flags: &["--realm", "client", "--tac", "java"],
        programs: &["javac"],
        typescript_client: false,
    };
    let project = initialized(workspace.path(), &case);
    assert!(
        project
            .join("client/services/JavaGreetingService.java")
            .is_file()
    );
    assert!(
        project
            .join("client/tests/JavaGreetingServiceTest.java")
            .is_file()
    );
    assert!(
        project
            .join("client/components/greeting/java/tac.html")
            .is_file()
    );
    if let Some(program) = case.missing_program() {
        eprintln!("skipping the Java checks: {program} is unavailable");
        return;
    }
    let tested = run(ty().args(["test", "--coverage"]).arg(&project));
    let report = text(&tested);
    assert!(tested.status.success(), "{report}");
    assert!(report.contains(" 0 failed."), "{report}");
    assert!(report.contains("called   greet"), "{report}");
    if !run(ty().arg("doctor").arg(&project)).status.success() {
        eprintln!("skipping the Java build: TeaVM is not installed");
        return;
    }
    let built = run(ty().arg("build").arg(&project));
    assert!(built.status.success(), "{}", text(&built));
    assert!(
        project
            .join("dist/.tachyon/services/java/services.wasm")
            .is_file()
    );
}

#[test]
fn help_lists_every_language_that_init_and_add_accept() {
    use tachyon_core::{Choice as _, TacLanguage, YonLanguage};
    let help = |arguments: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_ty"))
            .args(arguments)
            .output()
            .expect("the ty process should start");
        String::from_utf8_lossy(&output.stdout).replace(['\n', ' '], "")
    };
    let init = help(&["init", "--help"]);
    let add = help(&["add", "language", "--help"]);
    let tac = TacLanguage::flag_names().replace(' ', "");
    let yon = YonLanguage::flag_names().replace(' ', "");
    assert!(init.contains(&tac), "{tac}: {init}");
    assert!(init.contains(&yon), "{yon}: {init}");
    assert!(add.contains(&format!("{tac}forclient")), "{tac}: {add}");
    assert!(add.contains(&format!("{yon}forserver")), "{yon}: {add}");
}
