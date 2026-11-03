#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};

struct LinuxFilePickerTests;

impl LinuxFilePickerTests {
    #[cfg(target_os = "linux")]
    const NOTES: &'static str = "Quarterly notes é漢";
    #[cfg(target_os = "linux")]
    const DATA: &'static str = "{\"rows\":3}";
    const SERVICE: &'static str = r#"
#[Service]
#[derive(Default)]
struct ImportService {
    file_picker: FilePicker,
}

impl ImportService {
    pub fn pick(&self) -> String {
        let available = self.file_picker.available();
        let options = FilePickerOptions { multiple: false, types: vec![String::from("application/json")] };
        let chosen = self.file_picker.open(&options);
        let Some(handle) = chosen.handle else {
            return format!("{available}|{}", chosen.status.as_str());
        };
        let read = self.file_picker.read_text(&handle);
        let released = self.file_picker.release(&handle);
        let name = chosen.name.unwrap_or_default();
        let text = read.text.unwrap_or_default();
        format!("{available}|{name}|{text}|{}", released.status.as_str())
    }
}
"#;
    const PAGE_SCRIPT: &'static str = r"__PAGE_RUNTIME__
const picker = TacNativeCapabilities.filePicker();
const results = ['available=' + picker.available()];
const chosen = await picker.open({ types: ['text/plain'] });
results.push('open=' + chosen.status + ':' + chosen.name);
const read = await picker.readText(chosen.handle);
results.push('read=' + read.status + ':' + read.text);
results.push('release=' + (await picker.release(chosen.handle)).status);
results.push('again=' + (await picker.readText(chosen.handle)).status);
results.push('cancel=' + (await picker.open({ types: [] })).status);
results.push('multiple=' + (await picker.open({ multiple: true })).status);
return results.join(',');";
    const PAGE_EXPECTED: &'static str = "available=true,open=selected:notes.txt,read=read:Quarterly notes é漢,release=released,again=invalid,cancel=cancelled,multiple=invalid";
    const SERVICE_SCRIPT: &'static str = r"const owner = '00000000-0000-4000-8000-000000000061';
const post = (payload) => window.webkit.messageHandlers.tachyon.postMessage({capability: 'companion.invoke', payload});
const send = async (op, name, args) => JSON.parse(await post(JSON.stringify({route: '/', service: 'import', owner, op, name, args})));
const init = await send('init', '', []);
if (init.error) return 'init ' + init.error;
return 'pick=' + (await send('call', 'pick', [])).value;";
    const SERVICE_EXPECTED: &'static str = "pick=true|data.json|{\"rows\":3}|released";

    fn write(path: &Path, source: &[u8]) -> PathBuf {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("fixture file");
        path.to_path_buf()
    }

    fn stage(root: &Path) -> PathBuf {
        let input = NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Rust,
            source: Self::write(
                &root.join("client/services/ImportService.rs"),
                Self::SERVICE.as_bytes(),
            ),
            service: String::from("import"),
            class: String::from("ImportService"),
            web_assembly: false,
            application_provider: false,
        };
        super::rust::stage(&[input], root, "org.tachyon.file-picker")
            .expect("staged Rust services")
            .expect("native Rust source")
    }

    #[cfg(target_os = "linux")]
    fn choices(folder: &Path) {
        Self::write(&folder.join("notes.txt"), Self::NOTES.as_bytes());
        Self::write(&folder.join("data.json"), Self::DATA.as_bytes());
        Self::write(&folder.join("large.txt"), &vec![b'a'; 65_537]);
        Self::write(&folder.join("binary.txt"), &[0xff, 0xfe, 0x00, 0x41]);
    }

    fn fixture(folder: &Path) -> String {
        let folder = folder.to_str().expect("folder");
        concat!(
            include_str!("linux_wayland_input.c"),
            include_str!("linux_file_picker_fixture.c")
        )
        .replace("__PICKER_FOLDER__", folder)
        .replace(
            "__PAGE_SCRIPT__",
            &super::host::c_string_escape(&Self::page_script()),
        )
        .replace("__PAGE_EXPECTED__", Self::PAGE_EXPECTED)
        .replace(
            "__SERVICE_SCRIPT__",
            &super::host::c_string_escape(Self::SERVICE_SCRIPT),
        )
        .replace(
            "__SERVICE_EXPECTED__",
            &super::host::c_string_escape(Self::SERVICE_EXPECTED),
        )
    }

    fn page_script() -> String {
        let runtime = [
            include_str!("../template/tac-web.js"),
            include_str!("../template/tac-native.js"),
        ]
        .join("\n");
        Self::PAGE_SCRIPT.replace("__PAGE_RUNTIME__", &runtime)
    }

    #[cfg(target_os = "linux")]
    fn folder() -> PathBuf {
        std::env::temp_dir().join(format!("ty-picker-{}", std::process::id()))
    }
}

#[test]
fn the_file_picker_fixture_stages_a_granted_service_and_the_page_provider() {
    let root = tempfile::tempdir().expect("fixture");
    let source =
        std::fs::read_to_string(LinuxFilePickerTests::stage(root.path())).expect("companion");
    assert!(source.contains("struct ImportService"));
    let fixture = LinuxFilePickerTests::fixture(Path::new("/tmp/ty-picker-7"));
    assert!(fixture.contains("TacNativeCapabilities.filePicker()"));
    assert!(fixture.contains("#define TACHYON_PICKER_FOLDER \"/tmp/ty-picker-7\""));
    assert!(fixture.contains("XTestFakeKeyEvent"));
    assert!(fixture.contains("\"NotifyKeyboardKeysym\""));
    assert!(!fixture.contains("__PAGE_"));
    assert!(!fixture.contains("__SERVICE_"));
    let mut application = super::linux_host_tests::LinuxHostTests::application(
        super::native_menu_config::NativeMenuConfig::default(),
    );
    application.native_capabilities = vec![String::from("filePicker")];
    let host = super::linux::c_source(
        &application,
        &super::linux_host_tests::LinuxHostTests::index(),
    );
    assert!(host.contains("static gboolean g_file_picker_granted = TRUE;"));
    assert!(host.contains("TACHYON_NATIVE_SHIM, TACHYON_FILE_PICKER_PAGE);"));
    assert!(host.contains("answer = tachyon_capability_page_call(capability, payload);"));
    assert!(host.contains(
        "picked = tachyon_file_picker_call(operation + strlen(\"filePicker.\"), request);"
    ));
    assert!(!host.contains("__NATIVE_FILE_PICKER__"));
}

#[test]
fn the_linux_host_answers_webassembly_pickers_through_the_page_route() {
    let mut application = super::linux_host_tests::LinuxHostTests::application(
        super::native_menu_config::NativeMenuConfig::default(),
    );
    application.native_capabilities = vec![String::from("filePicker")];
    let host = super::linux::c_source(
        &application,
        &super::linux_host_tests::LinuxHostTests::index(),
    );
    let scheme = host
        .split("static void tachyon_scheme_request(")
        .nth(1)
        .expect("scheme handler");
    assert!(scheme.starts_with(
        "WebKitURISchemeRequest *request, gpointer data) {\n  (void)data;\n  if (tachyon_capability_route(request)) return;"
    ));
    for check in [
        "#define TACHYON_CAPABILITY_ROUTE \"/__tachyon/capability/\"",
        "g_strcmp0(webkit_uri_scheme_request_get_http_method(request), \"POST\") == 0",
        "g_str_has_prefix(capability, \"filePicker.\")",
        "soup_message_headers_get_one(headers, \"X-Tachyon-Capability\")",
        "webkit_uri_scheme_request_get_web_view(request) == g_view && tachyon_trusted_view(g_view)",
        "g_idle_add(tachyon_capability_route_run, call);",
        "tachyon_capability_page_call(call->capability, call->payload)",
        "globalThis.__tachyonNativeFilePickerRoute = true;",
    ] {
        assert!(host.contains(check), "{check}");
    }
    assert!(!host.contains("__NATIVE_CAPABILITY_ROUTE__"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, and an X11 display (Xvfb) with libXtst or a headless mutter Wayland session (scripts/linux/compositor-test.sh); run explicitly on the Linux lane"]
async fn linux_generated_host_opens_the_gtk_file_dialog_for_a_page_and_a_rust_service() {
    let root = tempfile::tempdir().expect("fixture");
    let folder = LinuxFilePickerTests::folder();
    LinuxFilePickerTests::choices(&folder);
    let source = LinuxFilePickerTests::stage(root.path());
    let mut application = super::linux_host_tests::LinuxHostTests::application(
        super::native_menu_config::NativeMenuConfig::default(),
    );
    application.native_capabilities = vec![String::from("filePicker")];
    let executable = super::linux_host_tests::LinuxHostTests::compile_application(
        root.path(),
        &source,
        &application,
        &LinuxFilePickerTests::fixture(&folder),
    )
    .await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[]).await;
    std::fs::remove_dir_all(&folder).expect("chosen files");
    let output = output.expect("actual generated Linux host file picker");
    assert!(output.contains("linux file picker passed"), "{output}");
    println!("{output}");
}
