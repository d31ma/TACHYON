#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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

fn write_phase4_project(project: &Path) {
    write(
        &project.join("client/manifest.json"),
        r#"{"name":"PhaseFour","start_url":"/","tac":{"application":{"id":"dev.tachyon.phase-four","version":"0.0.1"},"visibility":{"check":false}}}"#,
    );
    write(&project.join("client/pages/tac.html"), "<phase-four />");
    write(
        &project.join("client/components/phase/four/tac.html"),
        r#"
    <w-app-bar role="banner" aria-label="Primary navigation"><strong>Native Catalog</strong></w-app-bar>
    <main aria-label="Phase Four demo">
      <h1>Native Catalog</h1>
      <p>Native Tac content</p>
      <status-card><span>Slotted native content</span></status-card>
      <button
        aria-label="Increase count"
        data-tachyon-action="increment:count"
      >Increase</button>
      <output
        aria-label="Current count"
        data-tachyon-bind="count"
        data-tachyon-state="0"
      >0</output>
      <input
        aria-label="Customer name"
        data-tachyon-bind="customer"
        data-tachyon-state=""
        placeholder="Name"
      >
      <fancy-chart aria-label="Sales chart"><p>Web chart fallback</p></fancy-chart>
      <footer><small>Native sibling after fallback</small></footer>
    </main>
"#,
    );
    write(
        &project.join("client/components/status/card/tac.html"),
        "<article aria-label=\"Status card\"><slot></slot></article>",
    );
}

#[test]
fn unknown_build_targets_are_rejected_before_project_mutation() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view1 />",
    );
    write(
        &project.path().join("client/components/test/view1/tac.html"),
        "<main>safe</main>",
    );
    let output = run(ty()
        .arg("build")
        .arg(project.path())
        .args(["--target", "solaris"]));
    assert!(!output.status.success());
    assert!(!project.path().join("dist").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn a_mixture_stages_only_the_service_language_that_answers_the_target() {
    let project = tempfile::tempdir().expect("project");
    write_phase4_project(project.path());
    write(
        &project.path().join("client/components/phase/four/tac.html"),
        "<main><p>{phaseFour.runtime}</p></main>",
    );
    write(
        &project
            .path()
            .join("client/services/PhaseFourService.swift"),
        "@Service\nfinal class PhaseFourService {\n  let runtime = \"Swift\"\n}\n",
    );
    write(
        &project.path().join("client/services/PhaseFourService.rs"),
        "#[Service]\npub struct PhaseFourService {\n    pub runtime: &'static str,\n}\n\n\
         impl PhaseFourService {\n    pub fn new() -> Self {\n        \
         Self { runtime: \"Rust\" }\n    }\n}\n",
    );

    let output = run(ty()
        .arg("build")
        .arg(project.path())
        .args(["--target", "macos"]));
    assert!(output.status.success(), "{}", stderr(&output));

    let staged = project.path().join("dist/macos/project");
    assert!(staged.join("TachyonCompanion.swift").is_file());
    assert!(!staged.join("companion.rs").exists());
    let host = fs::read_to_string(staged.join("TachyonHost.swift")).expect("host");
    assert!(host.contains("tacNativeInvoke(payload)"), "{host}");
    assert!(!host.contains("tacRustInvoke"), "{host}");
}

#[cfg(target_os = "macos")]
#[test]
fn multiple_native_services_compile_and_swift_state_is_isolated() {
    let project = tempfile::tempdir().expect("project");
    write_multiple_native_services(&project);
    let output = run(ty()
        .arg("build")
        .arg(project.path())
        .args(["--target", "macos"]));
    assert!(output.status.success(), "{}", stderr(&output));
    let stage = project.path().join("dist/macos/project");
    let probe = stage.join("Probe.swift");
    write_native_probe(&probe);
    let executable = stage.join("probe");
    let json_helper = stage.join("AppleJSON.swift");
    write(
        &json_helper,
        include_str!("../../../tachyon-core/src/native/apple_json.swift"),
    );
    let compiled = run(Command::new("/usr/bin/xcrun")
        .arg("swiftc")
        .arg("-O")
        .arg(&json_helper)
        .arg(stage.join("TachyonCompanion.swift"))
        .arg(&probe)
        .arg("-o")
        .arg(&executable));
    assert!(compiled.status.success(), "{}", stderr(&compiled));
    let evidence = run(&mut Command::new(executable));
    assert!(evidence.status.success(), "{}", stderr(&evidence));
    let host = fs::read_to_string(stage.join("TachyonHost.swift")).expect("host");
    assert!(host.contains("tacServiceKnown") && host.contains("tacRustInvoke"));
}

#[cfg(target_os = "macos")]
fn write_multiple_native_services(project: &tempfile::TempDir) {
    for (route, component, class, instance) in [
        ("", "first", "NativeFirstService", "nativeFirst"),
        ("second/", "second", "NativeSecondService", "nativeSecond"),
    ] {
        write(
            &project.path().join(format!("client/pages/{route}tac.html")),
            &format!("<native-{component} />"),
        );
        write(
            &project
                .path()
                .join(format!("client/components/native/{component}/tac.html")),
            &format!("<main>{{{instance}.count}} {{{instance}.doubled()}}</main>"),
        );
        write(
            &project
                .path()
                .join(format!("client/services/{class}.swift")),
            &format!(
                "@Service\nfinal class {class} {{\n var count: Int = 0\n func doubled() -> Int {{ count * 2 }}\n}}\n"
            ),
        );
    }
    write(
        &project.path().join("client/pages/rust/tac.html"),
        "<native-rust />",
    );
    write(
        &project
            .path()
            .join("client/components/native/rust/tac.html"),
        "<main>{nativeRust.count}</main>",
    );
    write(
        &project.path().join("client/services/NativeRustService.rs"),
        "#[Service]\n#[derive(Default)]\nstruct NativeRustService {\n count: i64,\n}\nimpl NativeRustService {\n fn doubled(&self) -> i64 { self.count * 2 }\n}\n",
    );
}

#[cfg(target_os = "macos")]
fn write_native_probe(probe: &Path) {
    write(
        probe,
        r#"import Foundation
@main struct Probe {
 static func main() throws {
  func invoke(_ service: String, _ owner: String, _ operation: String, _ fields: [String: Any] = [:]) throws -> [String: Any] {
   var request = fields
   request["route"] = "/"; request["service"] = service; request["owner"] = owner; request["op"] = operation
   let raw = String(decoding: try JSONSerialization.data(withJSONObject: request), as: UTF8.self)
   return try JSONSerialization.jsonObject(with: Data(tacNativeInvoke(raw).utf8)) as! [String: Any]
  }
  let ownerA = "00000000-0000-4000-8000-000000000001"
  let ownerB = "00000000-0000-4000-8000-000000000002"
  let ownerC = "00000000-0000-4000-8000-000000000003"
  let first = try invoke("nativeFirst", ownerA, "init")
  precondition(first["value"] is [String: Any])
  _ = try invoke("nativeFirst", ownerA, "set", ["name":"count", "value":7])
  let doubled = try invoke("nativeFirst", ownerA, "call", ["name":"doubled", "args":[]])
  precondition(doubled["value"] as? Int == 14)
  _ = try invoke("nativeSecond", ownerB, "init")
  let second = try invoke("nativeSecond", ownerB, "get", ["name":"count"])
  precondition(second["value"] as? Int == 0)
  let coexisting = try invoke("nativeFirst", ownerA, "get", ["name":"count"])
  precondition(coexisting["value"] as? Int == 7)
  _ = try invoke("nativeFirst", ownerC, "init")
  let sibling = try invoke("nativeFirst", ownerC, "get", ["name":"count"])
  precondition(sibling["value"] as? Int == 0)
  _ = try invoke("nativeFirst", ownerA, "dispose")
  let stale = try invoke("nativeFirst", ownerA, "get", ["name":"count"])
  precondition(stale["error"] != nil)
  let staleDispose = try invoke("nativeFirst", ownerA, "dispose")
  precondition(staleDispose["error"] != nil)
  let stillActive = try invoke("nativeFirst", ownerC, "get", ["name":"count"])
  precondition(stillActive["value"] as? Int == 0)
  let unknown = try invoke("nativeMissing", ownerC, "init")
  precondition(unknown["code"] as? String == "TY_NATIVE_SERVICE")
  print("Swift native service activation, coexistence, isolation and stale-owner rejection passed")
 }
}
"#,
    );
}

#[test]
fn a_swift_only_service_is_compiled_to_webassembly_for_the_web() {
    let project = tempfile::tempdir().expect("project");
    write_phase4_project(project.path());
    write(
        &project
            .path()
            .join("client/services/PhaseFourService.swift"),
        "@Service\nfinal class PhaseFourService {\n  let runtime = \"Swift\"\n}\n",
    );

    let output = run(ty()
        .arg("bundle")
        .arg(project.path())
        .env_remove("TAC_SWIFT_WASM_SDK"));
    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(
        message.contains("Cannot build the Swift web services"),
        "{message}"
    );
    assert!(message.contains("TAC_SWIFT_WASM_SDK"), "{message}");

    write(
        &project.path().join("client/services/PhaseFourService.js"),
        "@Service()\nexport default class PhaseFourService {\n  runtime = 'JavaScript'\n}\n",
    );
    let mixed = run(ty().arg("bundle").arg(project.path()));
    assert!(mixed.status.success(), "{}", stderr(&mixed));
}

#[test]
fn browser_services_keep_constants_inside_their_class() {
    let project = tempfile::tempdir().expect("project");
    write_phase4_project(project.path());
    write(
        &project.path().join("client/shared/nothing.js"),
        "export {};\n",
    );
    write(
        &project.path().join("client/components/phase/four/tac.html"),
        "<main><output>{phaseFour.count}</output></main>",
    );
    write(
        &project.path().join("client/pages/tac.html"),
        "<phase-four hydrate=\"load\" />",
    );
    write(
        &project.path().join("client/services/PhaseFourService.js"),
        "import '/shared/nothing.js'\n\n@Service()\nexport default class PhaseFourService {\n  static LIMIT = 3\n  count = PhaseFourService.LIMIT\n}\n",
    );

    let output = run(ty().arg("bundle").arg(project.path()));
    assert!(output.status.success(), "{}", stderr(&output));
    let module = fs::read_to_string(
        project
            .path()
            .join("dist/web/.tachyon/services/PhaseFourService.js"),
    )
    .expect("module");
    assert!(module.contains("static LIMIT = 3"));
    assert!(
        !project
            .path()
            .join("dist/web/.tachyon/components/phase-four.js")
            .exists()
    );
}

fn rust_web_target_installed() -> bool {
    let Ok(output) = Command::new("rustc")
        .args([
            "--print",
            "target-libdir",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .output()
    else {
        return false;
    };
    output.status.success() && Path::new(String::from_utf8_lossy(&output.stdout).trim()).is_dir()
}

fn dotnet_web_workload_installed() -> bool {
    let Ok(output) = Command::new("dotnet").args(["workload", "list"]).output() else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).contains("wasm-tools")
}

fn skip_without(toolchain_ready: bool, toolchain: &str) -> bool {
    if toolchain_ready {
        return false;
    }
    eprintln!("skipping: {toolchain} is unavailable");
    true
}

fn assert_staged_as_webassembly(language: &str, source: &str, target: &str, engine: &str) {
    let project = tempfile::tempdir().expect("project");
    write_phase4_project(project.path());
    write(
        &project
            .path()
            .join(format!("client/services/PhaseFourService.{language}")),
        source,
    );
    let output = run(ty()
        .arg("bundle")
        .arg(project.path())
        .args(["--target", target, "--native"]));
    assert!(
        output.status.success(),
        "{language} on {target}: {}",
        stderr(&output)
    );
    let staged = project.path().join("dist").join(target);
    let services = staged.join("web/.tachyon/services");
    assert!(services.join(engine).is_file(), "{language} on {target}");
    assert!(
        services.join("PhaseFourService.js").is_file(),
        "{language} on {target}"
    );
    let host = fs::read_to_string(staged.join("tachyon.host.json")).expect("host descriptor");
    let host: serde_json::Value = serde_json::from_str(&host).expect("host json");
    assert_eq!(
        host["companions"],
        serde_json::json!([]),
        "{language} on {target}"
    );
}

#[test]
fn a_rust_service_is_webassembly_on_targets_that_do_not_link_it() {
    if skip_without(
        rust_web_target_installed(),
        "the wasm32-unknown-unknown Rust target",
    ) {
        return;
    }
    let source = "#[Service]\npub struct PhaseFourService {\n    pub count: i32,\n}\n\n\
                  impl PhaseFourService {\n    pub fn new() -> Self {\n        \
                  Self { count: 6 }\n    }\n}\n";
    for target in ["ios", "android"] {
        assert_staged_as_webassembly("rs", source, target, "tachyon-rust.js");
    }
}

#[test]
fn a_csharp_service_is_webassembly_on_targets_that_do_not_link_it() {
    if skip_without(
        dotnet_web_workload_installed(),
        "the .NET wasm-tools workload",
    ) {
        return;
    }
    let source = "[Service]\npublic class PhaseFourService\n{\n    public int Count = 6;\n}\n";
    assert_staged_as_webassembly("cs", source, "linux", "tachyon-csharp.js");
}

fn service_with_roles(language: &str, lifetime: &str) -> String {
    let lifetime = format!("Lifetime({{\"value\":\"{lifetime}\"}})");
    let (roles, body) = match language {
        "swift" => (
            format!("@Service\n@{lifetime}\n@NeedsAll\n"),
            "final class PhaseFourService {\n  var count: Int = 6\n}\n",
        ),
        "kt" => (
            format!("@Service\n@{lifetime}\n@NeedsAll\n"),
            "class PhaseFourService {\n  var count: Int = 6\n}\n",
        ),
        "js" | "ts" => (
            format!("@Service()\n@{lifetime}\n@NeedsAll\n"),
            "export default class PhaseFourService {\n  count = 6\n}\n",
        ),
        "dart" => (
            format!("@Service()\n@{lifetime}\n@NeedsAll()\n"),
            "class PhaseFourService {\n  int count = 6;\n}\n",
        ),
        "rs" => (
            format!("#[Service]\n#[{lifetime}]\n#[NeedsAll]\n"),
            "#[derive(Default)]\npub struct PhaseFourService {\n    pub count: i64,\n}\n",
        ),
        "go" => (
            format!("// @Service\n// @{lifetime}\n// @NeedsAll\n"),
            "type PhaseFourService struct {\n\tCount int64\n}\n",
        ),
        _ => (
            format!("[Service]\n[{lifetime}]\n[NeedsAll]\n"),
            "public class PhaseFourService\n{\n    public long Count = 6;\n}\n",
        ),
    };
    format!("// The count shown on the page.\n{roles}{body}")
}

fn bundle_service(language: &str, source: &str, target: &[&str]) -> (tempfile::TempDir, Output) {
    let project = tempfile::tempdir().expect("project");
    write_phase4_project(project.path());
    write(
        &project.path().join("client/components/phase/four/tac.html"),
        "<main><p>{phaseFour.count}</p></main>",
    );
    write(
        &project
            .path()
            .join(format!("client/services/PhaseFourService.{language}")),
        source,
    );
    let output = run(ty().arg("bundle").arg(project.path()).args(target));
    (project, output)
}

#[test]
fn swift_and_kotlin_services_accept_the_service_role_markers() {
    for (language, target) in [("swift", "ios"), ("swift", "macos"), ("kt", "android")] {
        let source = service_with_roles(language, "prototype");
        let (project, output) =
            bundle_service(language, &source, &["--native", "--target", target]);
        assert!(
            output.status.success(),
            "{language} on {target}: {}",
            stderr(&output)
        );
        if language == "swift" {
            let companion = project
                .path()
                .join(format!("dist/{target}/project/TachyonCompanion.swift"));
            let companion = fs::read_to_string(companion).expect("companion");
            assert!(companion.contains("final class PhaseFourService {"));
            assert!(!companion.contains("@Service"), "{companion}");
            assert!(!companion.contains("init(count"), "{companion}");
        }
    }
}

#[test]
fn a_bad_lifetime_is_refused_at_its_marker_in_every_tac_language() {
    for (language, target) in [
        ("js", &[][..]),
        ("ts", &[]),
        ("dart", &[]),
        ("rs", &[]),
        ("cs", &[]),
        ("go", &[]),
        ("swift", &["--native", "--target", "ios"]),
        ("kt", &["--native", "--target", "android"]),
    ] {
        let source = service_with_roles(language, "sometimes");
        let (_project, output) = bundle_service(language, &source, target);
        let message = stderr(&output);
        assert!(!output.status.success(), "{language}");
        assert!(
            message.contains("Lifetime.value must be singleton or prototype"),
            "{language}: {message}"
        );
        let lifetime = source.find("Lifetime(").expect("marker");
        let line_start = source[..lifetime].rfind('\n').expect("line") + 1;
        let marker_start = if language == "go" {
            line_start + "// ".len()
        } else {
            line_start
        };
        let location = format!("client/services/PhaseFourService.{language}:{marker_start}..");
        assert!(message.contains(&location), "{language}: {message}");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_bundle_hosts_the_applications_own_web_bundle() {
    let project = tempfile::tempdir().expect("project");
    write_phase4_project(project.path());

    let output = run(ty()
        .arg("build")
        .arg(project.path())
        .args(["--target", "macos"]));
    assert!(output.status.success(), "{}", stderr(&output));

    let root = project.path().join("dist/macos");
    let bundle = root.join("PhaseFour.app");
    assert!(bundle.join("Contents/MacOS/PhaseFour").is_file());
    assert!(bundle.join("Contents/Info.plist").is_file());
    assert!(root.join("project/TachyonHost.swift").is_file());
    assert!(root.join("artifact-manifest.json").is_file());
    assert!(root.join("native-index.json").is_file());
    assert!(root.join("web/index.html").is_file());
    assert!(
        bundle
            .join("Contents/Resources/WebBundle/index.html")
            .is_file()
    );

    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("native-index.json")).expect("route index"))
            .expect("route index JSON");
    assert_eq!(index["contract_version"], 2);
    assert_eq!(index["entry_route"], "/");
    assert_eq!(index["entry_document"], "index.html");

    let artifact: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("artifact-manifest.json")).expect("artifact manifest"),
    )
    .expect("artifact JSON");
    assert_eq!(artifact["target"]["os"], "macos");
    assert_contract("artifact-manifest", &artifact);
    let descriptor: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("tachyon.host.json")).expect("native host descriptor"),
    )
    .expect("native host JSON");
    assert_contract("native-host", &descriptor);
    assert_eq!(artifact["contracts"]["native_host"], 3);
    assert!(artifact["contracts"].get("native_ui").is_none());
    assert!(
        artifact["outputs"]
            .as_array()
            .is_some_and(|value| !value.is_empty())
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_native_failures_preserve_the_last_complete_application() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/manifest.json"),
        r#"{"name":"Rollback","start_url":"/","tac":{"application":{"id":"dev.tachyon.rollback","version":"0.0.1"}}}"#,
    );
    write(
        &project.path().join("client/pages/tac.html"),
        "<app-view />",
    );
    let source = project.path().join("client/components/app/view/tac.html");
    write(
        &source,
        r#"<main><button aria-label="Increase" data-tachyon-action="increment:count">Increase</button><output data-tachyon-bind="count" data-tachyon-state="0">0</output></main>"#,
    );
    let first = run(ty()
        .arg("build")
        .arg(project.path())
        .args(["--target", "macos"]));
    assert!(first.status.success(), "{}", stderr(&first));
    let executable = project
        .path()
        .join("dist/macos/Rollback.app/Contents/MacOS/Rollback");
    let published = fs::read(&executable).expect("published executable");

    write(
        &project.path().join("client/manifest.json"),
        r#"{"name":"Rollback","start_url":"/missing","tac":{"application":{"id":"dev.tachyon.rollback","version":"0.0.1"}}}"#,
    );
    let failed = run(ty()
        .arg("build")
        .arg(project.path())
        .args(["--target", "macos"]));
    assert!(!failed.status.success());
    assert!(stderr(&failed).contains("TY1601"), "{}", stderr(&failed));
    assert_eq!(
        fs::read(executable).expect("preserved executable"),
        published
    );
}
