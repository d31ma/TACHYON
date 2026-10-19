#![allow(clippy::expect_used)]

use super::config::WindowConfiguration;
use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};

struct LinuxWindowControlTests;

impl LinuxWindowControlTests {
    const GRANTS: [&'static str; 3] = ["title", "resize", "minimize"];
    const SERVICE: &'static str = r"
#[Service]
#[derive(Default)]
struct WindowService {
    window_controls: WindowControls,
}

impl WindowService {
    pub fn granted(&self, control: String) -> bool {
        self.window_controls.granted(&control).unwrap_or(false)
    }

    pub fn rename(&self, title: String) -> String {
        self.window_controls.set_title(&title).err().unwrap_or_default()
    }

    pub fn resize(&self, width: i64, height: i64) -> String {
        let width = u32::try_from(width).unwrap_or(0);
        let height = u32::try_from(height).unwrap_or(0);
        self.window_controls.resize(width, height).err().unwrap_or_default()
    }

    pub fn minimize(&self) -> String {
        self.window_controls.minimize().err().unwrap_or_default()
    }

    pub fn close(&self) -> String {
        self.window_controls.close().err().unwrap_or_default()
    }

    pub fn maximize(&self) -> String {
        self.window_controls.maximize().err().unwrap_or_default()
    }

    pub fn fullscreen(&self) -> String {
        self.window_controls.fullscreen().err().unwrap_or_default()
    }
}
";
    #[cfg(target_os = "linux")]
    const ALL_GRANTS: [&'static str; 6] = [
        "title",
        "resize",
        "minimize",
        "maximize",
        "fullscreen",
        "close",
    ];
    const GRANT_SCRIPT: &'static str = r"const owner = '00000000-0000-4000-8000-000000000052';
const post = (payload) => window.webkit.messageHandlers.tachyon.postMessage({capability: 'companion.invoke', payload});
const send = async (op, name, args) => JSON.parse(await post(JSON.stringify({route: '/', service: 'window', owner, op, name, args})));
if (globalThis.__grantCalls === undefined) globalThis.__grantCalls = ['maximize', 'fullscreen', 'fullscreen', 'close'];
if (globalThis.__grantReady === undefined) {
  const init = await send('init', '', []);
  if (init.error) return 'init ' + init.error;
  globalThis.__grantReady = true;
}
const control = globalThis.__grantCalls.shift();
return control + '=' + (await send('call', control, [])).value;";
    const SCRIPT: &'static str = r"const owner = '00000000-0000-4000-8000-000000000051';
const post = (payload) => window.webkit.messageHandlers.tachyon.postMessage({capability: 'companion.invoke', payload});
const send = async (op, name, args) => JSON.parse(await post(JSON.stringify({route: '/', service: 'window', owner, op, name, args})));
if (globalThis.__windowStep === undefined) {
  const init = await send('init', '', []);
  if (init.error) return 'init ' + init.error;
  globalThis.__windowStep = 1;
  const results = [];
  for (const control of ['title', 'resize', 'minimize', 'close', 'fullscreen']) results.push(control + '=' + (await send('call', 'granted', [control])).value);
  results.push('rename=' + (await send('call', 'rename', ['Quarterly report é漢😀'])).value);
  results.push('long=' + (await send('call', 'rename', ['x'.repeat(1025)])).value);
  results.push('resize=' + (await send('call', 'resize', [800, 600])).value);
  results.push('small=' + (await send('call', 'resize', [63, 600])).value);
  results.push('close=' + (await send('call', 'close', [])).value);
  return results.join(',');
}
return 'minimize=' + (await send('call', 'minimize', [])).value;";
    const PAGE_SCRIPT: &'static str = r"__PAGE_RUNTIME__
const failures = [];
let heard = null;
const failure = new Promise((resolve) => { heard = resolve; });
TacWebApis.events = { send: (owner, name, value) => { failures.push(name + ' ' + value.api + '.' + value.call + ': ' + value.message); heard(); } };
const controls = globalThis[Symbol.for('tachyon.tac.web')].windowControls;
controls.setTitle('Page title é漢');
controls.resize(700, 500);
globalThis.__tachyonWindowControls = Object.freeze([...globalThis.__tachyonWindowControls, 'close']);
controls.close();
await Promise.race([failure, new Promise((resolve) => setTimeout(resolve, 5000))]);
return 'title=' + controls.granted('title') + ',close=' + controls.granted('close') + ',' + failures.join(';');";
    const PAGE_EXPECTED: &'static str =
        "title=true,close=true,BrowserCallFailed windowControls.close: denied";
    const EXPECTED: &'static str = "title=true,resize=true,minimize=true,close=false,fullscreen=false,rename=,long=unavailable,resize=,small=invalid,close=denied";

    fn write(path: &Path, source: &str) -> PathBuf {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("fixture source");
        path.to_path_buf()
    }

    fn stage(root: &Path) -> PathBuf {
        let input = NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Rust,
            source: Self::write(
                &root.join("client/services/WindowService.rs"),
                Self::SERVICE,
            ),
            service: String::from("window"),
            class: String::from("WindowService"),
            web_assembly: false,
            application_provider: false,
        };
        super::rust::stage(&[input], root, "org.tachyon.window-controls")
            .expect("staged Rust services")
            .expect("native Rust source")
    }

    fn window() -> WindowConfiguration {
        WindowConfiguration {
            controls: Self::GRANTS.map(String::from).to_vec(),
            ..WindowConfiguration::default()
        }
    }

    fn fixture() -> String {
        include_str!("linux_window_control_fixture.c")
            .replace(
                "__WINDOW_SCRIPT__",
                &super::host::c_string_escape(Self::SCRIPT),
            )
            .replace("__WINDOW_EXPECTED__", Self::EXPECTED)
            .replace(
                "__PAGE_SCRIPT__",
                &super::host::c_string_escape(&Self::page_script()),
            )
            .replace("__PAGE_EXPECTED__", Self::PAGE_EXPECTED)
    }

    #[cfg(target_os = "linux")]
    fn granted_window() -> WindowConfiguration {
        WindowConfiguration {
            controls: Self::ALL_GRANTS.map(String::from).to_vec(),
            ..WindowConfiguration::default()
        }
    }

    fn grant_fixture() -> String {
        include_str!("linux_window_grant_fixture.c").replace(
            "__GRANT_SCRIPT__",
            &super::host::c_string_escape(Self::GRANT_SCRIPT),
        )
    }

    fn page_script() -> String {
        Self::PAGE_SCRIPT.replace("__PAGE_RUNTIME__", include_str!("../template/tac-web.js"))
    }
}

#[test]
fn the_window_control_fixture_stages_a_granted_service_and_a_real_bridge_script() {
    let root = tempfile::tempdir().expect("fixture");
    let source =
        std::fs::read_to_string(LinuxWindowControlTests::stage(root.path())).expect("companion");
    assert!(source.contains("struct WindowService"));
    let host = super::linux_window_controls::LinuxWindowControls::source(
        &LinuxWindowControlTests::window(),
    );
    assert_eq!(host.matches("return TRUE;").count(), 3);
    assert!(!host.contains("strcmp(control, \"close\") == 0) return TRUE;"));
    let fixture = LinuxWindowControlTests::fixture();
    assert!(fixture.contains("window.webkit.messageHandlers.tachyon.postMessage"));
    assert!(fixture.contains("static installed = TacWebApis.install()"));
    assert!(!fixture.contains("__WINDOW_"));
    assert!(!fixture.contains("__PAGE_"));
    let granted = LinuxWindowControlTests::grant_fixture();
    assert!(granted.contains("__grantCalls"));
    assert!(!granted.contains("__GRANT_"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files and an X11 display with a window manager (xfwm4 under Xvfb on the WSL lane); run explicitly on the Linux lane"]
async fn linux_generated_host_drives_granted_window_controls_and_refuses_a_denied_one() {
    let root = tempfile::tempdir().expect("fixture");
    let source = LinuxWindowControlTests::stage(root.path());
    let mut application = super::linux_host_tests::LinuxHostTests::application(
        super::native_menu_config::NativeMenuConfig::default(),
    );
    application.window = LinuxWindowControlTests::window();
    let executable = super::linux_host_tests::LinuxHostTests::compile_application(
        root.path(),
        &source,
        &application,
        &LinuxWindowControlTests::fixture(),
    )
    .await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated Linux host window controls");
    assert!(output.contains("linux window controls passed"), "{output}");
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files and a display with a window manager (xfwm4 under Xvfb, or headless mutter); run explicitly on the Linux lane"]
async fn linux_generated_host_maximizes_fills_the_screen_and_closes_when_granted() {
    let root = tempfile::tempdir().expect("fixture");
    let source = LinuxWindowControlTests::stage(root.path());
    let mut application = super::linux_host_tests::LinuxHostTests::application(
        super::native_menu_config::NativeMenuConfig::default(),
    );
    application.window = LinuxWindowControlTests::granted_window();
    let executable = super::linux_host_tests::LinuxHostTests::compile_application(
        root.path(),
        &source,
        &application,
        &LinuxWindowControlTests::grant_fixture(),
    )
    .await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated Linux host window grants");
    assert!(output.contains("linux window grants passed"), "{output}");
}
