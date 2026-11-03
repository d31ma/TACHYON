#![allow(clippy::expect_used)]

use crate::one_test_per_case::one_test_per_case;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn ty() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ty"))
}

fn run(command: &mut Command) -> Output {
    command.output().expect("the ty process should start")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
    fs::write(path, contents).expect("fixture source");
}

#[cfg(target_os = "macos")]
fn assert_contract(name: &str, instance: &serde_json::Value) {
    let contract = tachyon_contracts::find(name).expect("registered contract");
    let schema = tachyon_contracts::parse_schema(contract).expect("canonical schema");
    let validator = jsonschema::validator_for(&schema).expect("schema validator");
    assert!(
        validator.is_valid(instance),
        "{name} errors: {:?}",
        validator.iter_errors(instance).collect::<Vec<_>>()
    );
}

fn write_phase5_project(project: &Path, name: &str, id: &str) {
    write(
        &project.join("client/manifest.json"),
        &format!(
            r#"{{"name":"{name}","start_url":"/","tac":{{"application":{{"id":"{id}","version":"0.1.0"}}}}}}"#
        ),
    );
    write(&project.join("client/pages/tac.html"), "<test-view1 />");
    write(
        &project.join("client/components/test/view1/tac.html"),
        r#"<main aria-label="Phase Five demo">
  <h1>Phase Five</h1>
  <p>Cross-platform native adapters.</p>
  <button aria-label="Increase count" data-tachyon-action="increment:count">Add one</button>
  <output aria-label="Count" data-tachyon-bind="count" data-tachyon-state="0">0</output>
  <input aria-label="Your name" data-tachyon-bind="name" data-tachyon-state="" placeholder="Name">
  <details aria-label="More detail"><summary>More detail</summary><p>Disclosure content.</p></details>
  <x-chart aria-label="Sales chart"><p>Chart fallback</p></x-chart>
</main>
"#,
    );
}

#[cfg(target_os = "macos")]
fn assert_common_staging(root: &Path, target: &str) {
    assert!(root.join("artifact-manifest.json").is_file());
    assert!(root.join("native-index.json").is_file());
    assert!(root.join("web/index.html").is_file());

    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("native-index.json")).expect("route index"))
            .expect("route index JSON");
    assert_eq!(index["contract_version"], 2, "{target}");
    assert_eq!(index["entry_route"], "/", "{target}");
    assert_eq!(index["entry_document"], "index.html", "{target}");

    let host: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("tachyon.host.json")).expect("host descriptor"))
            .expect("host descriptor JSON");
    assert_eq!(host["target"], target);
    assert_eq!(host["renderMode"], "bundle");

    let artifact: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("artifact-manifest.json")).expect("artifact manifest"),
    )
    .expect("artifact JSON");
    assert_contract("artifact-manifest", &artifact);
    assert!(
        artifact["outputs"]
            .as_array()
            .is_some_and(|value| !value.is_empty())
    );
}

#[test]
fn every_native_target_publishes_under_its_own_directory() {
    for (target, directory) in [
        ("macos", "macos"),
        ("ios", "ios"),
        ("linux", "linux"),
        ("windows", "windows"),
        ("android", "android"),
    ] {
        let project = tempfile::tempdir().expect("project");
        write_phase5_project(project.path(), "Isolation", "dev.tachyon.isolation");
        let output = run(ty()
            .arg("build")
            .arg(project.path())
            .args(["--target", target]));
        let published = project.path().join("dist").join(directory);
        if output.status.success() {
            assert!(published.is_dir(), "{target} published nothing on success");
        } else {
            assert!(!published.exists(), "{target} published a failed build");
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn ios_bundle_is_launchable_and_hosts_the_application_web_bundle() {
    let project = tempfile::tempdir().expect("project");
    write_phase5_project(project.path(), "PhaseFive", "dev.tachyon.phase-five");

    let output = run(ty()
        .arg("build")
        .arg(project.path())
        .args(["--target", "ios"]));
    assert!(output.status.success(), "{}", stderr(&output));

    let root = project.path().join("dist/ios");
    let bundle = root.join("PhaseFive.app");
    assert!(bundle.join("PhaseFive").is_file(), "missing executable");
    assert!(bundle.join("Info.plist").is_file());
    assert!(bundle.join("NativeIndex.json").is_file());
    assert!(bundle.join("WebBundle/index.html").is_file());
    assert!(bundle.join("_CodeSignature").is_dir(), "bundle is unsigned");
    assert!(root.join("project/TachyonHost.swift").is_file());
    assert_common_staging(&root, "ios");

    let plist = fs::read_to_string(bundle.join("Info.plist")).expect("Info.plist");
    assert!(plist.contains("<string>iPhoneSimulator</string>"));
    assert!(plist.contains("<key>UILaunchScreen</key>"));
    assert!(plist.contains("dev.tachyon.phase-five"));

    let artifact: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("artifact-manifest.json")).expect("artifact manifest"),
    )
    .expect("artifact JSON");
    assert_eq!(artifact["target"]["os"], "ios");
    assert_eq!(artifact["target"]["abi"], "wkwebview-simulator");

    let swift = fs::read_to_string(root.join("project/TachyonHost.swift")).expect("Swift host");
    assert!(swift.contains("import UIKit"));
    assert!(!swift.contains("import AppKit"));
    assert!(swift.contains("WKScriptMessageHandlerWithReply"));
    assert!(swift.contains("tachyon-app://bundle/"));
}

#[cfg(target_os = "macos")]
#[test]
fn ios_and_macos_host_the_same_bundle() {
    let project = tempfile::tempdir().expect("project");
    write_phase5_project(project.path(), "Parity", "dev.tachyon.parity");
    for target in ["macos", "ios"] {
        let output = run(ty()
            .arg("build")
            .arg(project.path())
            .args(["--target", target]));
        assert!(output.status.success(), "{target}: {}", stderr(&output));
    }

    let read = |target: &str| -> serde_json::Value {
        serde_json::from_slice(
            &fs::read(
                project
                    .path()
                    .join("dist")
                    .join(target)
                    .join("native-index.json"),
            )
            .expect("route index"),
        )
        .expect("route index JSON")
    };
    assert_eq!(read("macos"), read("ios"));
}

#[test]
fn native_targets_reject_an_unreachable_entry_route_before_any_toolchain() {
    for target in ["macos", "ios", "linux", "windows", "android"] {
        let project = tempfile::tempdir().expect("project");
        write(
            &project.path().join("client/manifest.json"),
            r#"{"name":"Invalid","start_url":"/nowhere","tac":{"application":{"id":"dev.tachyon.invalid","version":"0.1.0"}}}"#,
        );
        write(
            &project.path().join("client/pages/tac.html"),
            "<test-view2 />",
        );
        write(
            &project.path().join("client/components/test/view2/tac.html"),
            r#"<main aria-label="Home"><h1>Home</h1></main>"#,
        );
        let output = run(ty().arg("build").arg(project.path()).args([
            "--target",
            target,
            "--diagnostic-format",
            "json",
        ]));
        assert!(
            !output.status.success(),
            "{target} accepted an unreachable entry route"
        );
        let text = stderr(&output);
        assert!(text.contains("TY1601"), "{target} diagnostic was {text}");
    }
}

struct NativeServiceProject;

impl NativeServiceProject {
    const TARGETS: [&'static str; 5] = ["macos", "ios", "linux", "windows", "android"];
    const PAGE: &'static str = "<counter-panel hydrate=\"load\" />";
    const PANEL: &'static str = "<main><button type=\"button\" on:click=\"counter.increment()\">Add one</button><output>{counter.count}</output></main>";

    fn write(project: &Path, manifest: &str, service: (&str, &str)) {
        let (file, source) = service;
        write(&project.join("client/manifest.json"), manifest);
        write(&project.join("client/pages/tac.html"), Self::PAGE);
        write(
            &project.join("client/components/counter/panel/tac.html"),
            Self::PANEL,
        );
        write(&project.join("client/services").join(file), source);
    }

    fn toolchain_ready(project: &Path, requirement: &str, provided_by_ci: &str) -> bool {
        let report = tachyon_core::doctor::DoctorReport::check(project).expect("doctor");
        let missing: Vec<_> = report
            .toolchains
            .iter()
            .filter(|row| row.requirement == requirement)
            .filter(|row| !matches!(row.state, tachyon_core::doctor::ToolchainState::Ready(_)))
            .map(|row| row.command.clone())
            .collect();
        assert!(
            missing.is_empty() || std::env::var_os("CI").is_none(),
            "{provided_by_ci}: {missing:?}"
        );
        if !missing.is_empty() {
            eprintln!("skipping: {requirement} need {missing:?}");
        }
        missing.is_empty()
    }

    fn web(workspace: &Path, target: &str) -> std::path::PathBuf {
        workspace.join("published").join(target).join("web")
    }

    fn assert_bundles(workspace: &Path, project: &Path, target: &str, runtime: &str) {
        let published = workspace.join("published");
        let output = run(ty()
            .arg("bundle")
            .arg(project)
            .args(["--target", target, "--skip-package"])
            .env("TAC_DIST_PATH", &published));
        assert!(output.status.success(), "{target}: {}", stderr(&output));
        let web = Self::web(workspace, target);
        assert!(
            web.join(".tachyon/services/CounterService.js").is_file(),
            "{target} bundled no service module"
        );
        assert!(
            web.join(runtime).is_file(),
            "{target} bundled no runtime at {runtime}"
        );
        assert!(
            published
                .join(target)
                .join("artifact-manifest.json")
                .is_file(),
            "{target} wrote no artifact manifest"
        );
    }
}

struct RubyNativeProject;

impl RubyNativeProject {
    const MANIFEST: &'static str = r#"{"name":"RubyNative","start_url":"/","tac":{"application":{"id":"dev.tachyon.ruby-native","version":"0.1.0"}}}"#;
    const SERVICE: (&'static str, &'static str) = (
        "CounterService.rb",
        "#[Service]\nclass CounterService\n  # @return [Integer]\n  attr_reader :count\n\n  def initialize\n    @count = 0\n  end\n\n  def increment\n    @count += 1\n  end\nend\n",
    );
    const RUNTIME: &'static str = ".tachyon/services/ruby/tachyon-ruby.wasm";
    const RUNTIME_REQUIREMENT: &'static str = "Ruby services on the web";

    fn has_ruby_runtime(project: &Path) -> bool {
        NativeServiceProject::toolchain_ready(
            project,
            Self::RUNTIME_REQUIREMENT,
            "CI installs Emscripten and Ruby to build the pinned mruby runtime, so the Ruby native bundles must run there",
        )
    }
}

fn ruby_service_bundles_into(target: &str) {
    let workspace = tempfile::tempdir().expect("workspace");
    let project = workspace.path().join("ruby-native");
    NativeServiceProject::write(
        &project,
        RubyNativeProject::MANIFEST,
        RubyNativeProject::SERVICE,
    );
    if !RubyNativeProject::has_ruby_runtime(&project) {
        return;
    }
    NativeServiceProject::assert_bundles(
        workspace.path(),
        &project,
        target,
        RubyNativeProject::RUNTIME,
    );
}

one_test_per_case!(
    a_ruby_service_bundles_into_every_native_target,
    ruby_service_bundles_into,
    all NativeServiceProject::TARGETS,
    [
        macos => "macos",
        ios => "ios",
        linux => "linux",
        windows => "windows",
        android => "android",
    ]
);

struct PythonNativeProject;

impl PythonNativeProject {
    const MANIFEST: &'static str = r#"{"name":"PythonNative","start_url":"/","tac":{"application":{"id":"dev.tachyon.python-native","version":"0.1.0"},"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}}"#;
    const SERVICE: (&'static str, &'static str) = (
        "CounterService.py",
        "@service\nclass CounterService:\n    count: int = 0\n\n    def increment(self) -> None:\n        self.count += 1\n",
    );
    const RUNTIME: &'static str = ".tachyon/services/python/314.0.7/pyodide.asm.wasm";
    const PYODIDE: &'static str = "python-web/314.0.7/pyodide.asm.wasm";

    fn has_pyodide() -> bool {
        let cached = tachyon_core::cache::root().join(Self::PYODIDE).is_file();
        assert!(
            cached || std::env::var_os("CI").is_none(),
            "CI installs the pinned Pyodide with scripts/ci/install-toolchains.mjs, so the Python native bundles must run there"
        );
        if !cached {
            eprintln!("skipping: the pinned Pyodide runtime is not in the Tachyon cache");
        }
        cached
    }
}

fn python_service_bundles_into(target: &str) {
    if !PythonNativeProject::has_pyodide() {
        return;
    }
    let workspace = tempfile::tempdir().expect("workspace");
    let project = workspace.path().join("python-native");
    NativeServiceProject::write(
        &project,
        PythonNativeProject::MANIFEST,
        PythonNativeProject::SERVICE,
    );
    NativeServiceProject::assert_bundles(
        workspace.path(),
        &project,
        target,
        PythonNativeProject::RUNTIME,
    );
}

one_test_per_case!(
    a_python_service_bundles_into_every_native_target,
    python_service_bundles_into,
    all NativeServiceProject::TARGETS,
    [
        macos => "macos",
        ios => "ios",
        linux => "linux",
        windows => "windows",
        android => "android",
    ]
);

struct FsharpNativeProject;

impl FsharpNativeProject {
    const MANIFEST: &'static str = r#"{"name":"FsharpNative","start_url":"/","tac":{"application":{"id":"dev.tachyon.fsharp-native","version":"0.1.0"}}}"#;
    const SERVICE: (&'static str, &'static str) = (
        "CounterService.fs",
        "[<Service>]\ntype CounterService() =\n    member val Count = 0 with get, set\n\n    member this.Increment() : unit =\n        this.Count <- this.Count + 1\n",
    );
    const FRAMEWORK: &'static str = ".tachyon/services/dotnet/_framework";
    const RUNTIME: &'static str = ".tachyon/services/dotnet/_framework/dotnet.js";
    const RUNTIME_REQUIREMENT: &'static str = "C# services on the web";
    const ASSEMBLIES: [&'static str; 2] = ["FsharpServices", "FSharp.Core"];

    fn has_dotnet_web_workload(project: &Path) -> bool {
        NativeServiceProject::toolchain_ready(
            project,
            Self::RUNTIME_REQUIREMENT,
            "CI installs the .NET SDK with the wasm-tools workload, so the F# native bundles must run there",
        )
    }

    fn assert_companion(workspace: &Path, target: &str) {
        let framework = NativeServiceProject::web(workspace, target).join(Self::FRAMEWORK);
        let names: Vec<String> = fs::read_dir(&framework)
            .expect("the .NET framework folder")
            .map(|entry| {
                entry
                    .expect("framework entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        for assembly in Self::ASSEMBLIES {
            let prefix = format!("{assembly}.");
            assert!(
                names.iter().any(|name| name.starts_with(&prefix)
                    && Path::new(name)
                        .extension()
                        .is_some_and(|extension| extension == "wasm")),
                "{target} bundled no {assembly} assembly: {names:?}"
            );
        }
        let boot = fs::read_to_string(framework.join("blazor.boot.json")).expect("boot manifest");
        assert!(
            boot.contains("\"FsharpServices.wasm\""),
            "{target} does not load the F# services assembly: {boot}"
        );
    }
}

fn fsharp_service_bundles_into(target: &str) {
    let workspace = tempfile::tempdir().expect("workspace");
    let project = workspace.path().join("fsharp-native");
    NativeServiceProject::write(
        &project,
        FsharpNativeProject::MANIFEST,
        FsharpNativeProject::SERVICE,
    );
    if !FsharpNativeProject::has_dotnet_web_workload(&project) {
        return;
    }
    NativeServiceProject::assert_bundles(
        workspace.path(),
        &project,
        target,
        FsharpNativeProject::RUNTIME,
    );
    FsharpNativeProject::assert_companion(workspace.path(), target);
}

one_test_per_case!(
    an_fsharp_service_bundles_into_every_native_target,
    fsharp_service_bundles_into,
    all NativeServiceProject::TARGETS,
    [
        macos => "macos",
        ios => "ios",
        linux => "linux",
        windows => "windows",
        android => "android",
    ]
);
