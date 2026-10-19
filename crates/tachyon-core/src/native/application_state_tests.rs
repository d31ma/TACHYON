#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::node_driver::NodeDriver;
use crate::project::NativeCompanion;
use crate::route_types::WebEventSources;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

struct ApplicationStateTests;

impl ApplicationStateTests {
    const RUNTIME: &'static str = include_str!("../template/tac-web.js");
    const DEADLINE: Duration = Duration::from_secs(30);
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
const target = new EventTarget()
globalThis.addEventListener = (type, listener) => target.addEventListener(type, listener)
globalThis.dispatchEvent = (event) => target.dispatchEvent(event)
globalThis.document = { visibilityState: 'hidden', addEventListener: () => {} }
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: true, language: 'en' } })
globalThis.matchMedia = () => ({ matches: false, addEventListener: () => {} })
globalThis.webkit = { messageHandlers: { tachyon: { postMessage: () => null } } }
;(0, eval)(await readFile('./shim.js', 'utf8'))
const heard = []
addEventListener('tac.application.state', (event) => heard.push(event.detail.state))
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const published = []
TacWebApis.watch({ publish: (name, value) => published.push([name, value]), send: () => {} })
for (const state of ['background', 'background', 'foreground', 'asleep']) globalThis.__tachyonApplicationState(state)
const states = published.filter(([name]) => name === 'ApplicationStateChanged').map(([, value]) => value.state)
console.log(JSON.stringify({ heard, states }))
";

    const KOTLIN_SERVICE: &'static str = "@Service\nclass StateService {\n    var state: String = \"\"\n\n    @OnEvent\n    fun hear(changed: ApplicationStateChanged) {\n        state = changed.state\n    }\n}\n";

    fn android_platform() -> Option<PathBuf> {
        std::env::var_os("ANDROID_HOME")
            .map(PathBuf::from)
            .map(|home| home.join("platforms/android-35/android.jar"))
            .filter(|platform| platform.is_file())
    }

    fn succeeds(command: &mut Command) {
        let output = command.output().expect("tool runs");
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn report() -> serde_json::Value {
        let root = tempfile::tempdir().expect("driver folder");
        let window = super::config::WindowConfiguration::default();
        std::fs::write(
            root.path().join("shim.js"),
            super::host::native_shim(&window, tachyon_contracts::NativeTarget::Linux),
        )
        .expect("shim");
        std::fs::write(root.path().join("tac-web.js"), Self::RUNTIME).expect("runtime");
        std::fs::write(root.path().join("drive.mjs"), Self::DRIVER).expect("driver");
        let mut command = NodeDriver::command();
        command.arg("drive.mjs").current_dir(root.path());
        let output = NodeDriver::output_within(&mut command, Self::DEADLINE).expect("node driver");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.lines().last().unwrap_or_default()).expect("report")
    }
}

#[test]
fn a_native_page_hears_each_host_state_change_once_and_never_an_unknown_state() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = ApplicationStateTests::report();
    assert_eq!(
        report["heard"],
        serde_json::json!(["background", "foreground"])
    );
    assert_eq!(
        report["states"],
        serde_json::json!(["foreground", "background", "foreground"])
    );
}

#[test]
fn natively_linked_swift_and_kotlin_get_the_application_state_model_only_when_they_name_it() {
    let swift = "@OnEvent\nfunc hear(_ changed: ApplicationStateChanged) {}\n";
    assert_eq!(
        WebEventSources::native_application_state("swift", swift),
        "\nstruct ApplicationStateChanged: Codable {\n    let state: String\n}\n"
    );
    let kotlin = "@OnEvent\nfun hear(changed: ApplicationStateChanged) {}\n";
    assert_eq!(
        WebEventSources::native_application_state("kt", kotlin),
        "\nclass ApplicationStateChanged(val state: String)\n"
    );
    assert!(WebEventSources::native_application_state("swift", "struct Plain {}\n").is_empty());
    let declared =
        format!("{swift}struct ApplicationStateChanged: Codable {{ let state: String }}\n");
    assert!(WebEventSources::native_application_state("swift", &declared).is_empty());
    assert!(WebEventSources::native_application_state("rs", swift).is_empty());
}

#[test]
fn the_android_host_and_a_kotlin_listener_for_the_application_state_compile() {
    let Some(platform) = ApplicationStateTests::android_platform() else {
        eprintln!("skipping: Android platform 35 is unavailable");
        return;
    };
    let root = tempfile::tempdir().expect("project");
    let host = root.path().join("StateHost.java");
    let program = format!(
        "import android.webkit.WebView;\nclass StateHost extends android.app.Activity {{ private WebView view; private void record(String event) {{}}\n{}\n}}",
        include_str!("android_application_state.java")
    );
    std::fs::write(&host, program).expect("host source");
    ApplicationStateTests::succeeds(
        Command::new("javac")
            .arg("-classpath")
            .arg(&platform)
            .arg("-d")
            .arg(root.path().join("host"))
            .arg(&host),
    );
    if Command::new("kotlinc").arg("-version").output().is_err() {
        eprintln!("skipping the Kotlin companion: kotlinc is unavailable");
        return;
    }
    let service = root.path().join("client/services/StateService.kt");
    std::fs::create_dir_all(service.parent().expect("folder")).expect("folder");
    std::fs::write(&service, ApplicationStateTests::KOTLIN_SERVICE).expect("service");
    let input = NativeCompanionInput {
        snapshot_root: root.path().to_path_buf(),
        language: NativeCompanion::Kotlin,
        source: service,
        service: String::from("state"),
        class: String::from("StateService"),
        web_assembly: false,
        application_provider: false,
    };
    let project = root.path().join("android");
    super::android::stage_companion(&[input], &project, "app.state", "app/state").expect("stage");
    ApplicationStateTests::succeeds(
        Command::new("kotlinc")
            .arg("-classpath")
            .arg(&platform)
            .arg(project.join("app/src/main/kotlin"))
            .arg("-d")
            .arg(root.path().join("classes")),
    );
}
