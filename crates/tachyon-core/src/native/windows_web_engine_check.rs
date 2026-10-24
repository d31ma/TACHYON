use super::config::NativeApplication;
use tachyon_contracts::NativeTarget;

pub(super) struct WindowsWebEngineCheck;

impl WindowsWebEngineCheck {
    const SOURCE: &'static str = include_str!("windows_web_engine_check.c");

    pub(super) fn source(application: &NativeApplication) -> String {
        let floor = application.minimum_system.floor(NativeTarget::Windows);
        Self::SOURCE.replace("__WEBVIEW2_FLOOR__", &floor.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::WindowsWebEngineCheck;
    use crate::native::config::NativeApplication;
    use crate::native::system_version::SystemVersion;
    use tachyon_contracts::NativeTarget;

    #[test]
    fn the_windows_host_checks_the_webview2_floor_before_it_opens_a_window() {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let mut application =
            NativeApplication::discover(root.path()).unwrap_or_else(|failure| panic!("{failure}"));
        let source = WindowsWebEngineCheck::source(&application);
        assert!(source.contains(r#"#define TACHYON_WEBVIEW2_FLOOR L"86.0""#));
        application.minimum_system = application
            .minimum_system
            .with_floor(NativeTarget::Windows, SystemVersion::new(119, 0));
        let raised = WindowsWebEngineCheck::source(&application);
        assert!(raised.contains(r#"#define TACHYON_WEBVIEW2_FLOOR L"119.0""#));
        assert!(raised.contains("GetAvailableCoreWebView2BrowserVersionString"));
        assert!(raised.contains("CompareBrowserVersions"));
        assert!(!raised.contains("__WEBVIEW2_FLOOR__"));
    }

    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler; compiles the WebView2 runtime check against the Win32 headers and the two WebView2 loader prototypes, not MSVC"]
    async fn the_webview2_check_compiles_against_the_win32_headers() {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let application =
            NativeApplication::discover(root.path()).unwrap_or_else(|failure| panic!("{failure}"));
        let harness = format!(
            "{}{}{}",
            include_str!("windows_web_engine_check_fixture.c"),
            WindowsWebEngineCheck::source(&application),
            "\nint main(void) {\n  if (!tachyon_web_engine_ready()) tachyon_show_web_engine_message();\n  return 0;\n}\n"
        );
        let source = root.path().join("check.c");
        std::fs::write(&source, harness).unwrap_or_else(|error| panic!("{error}"));
        let object = root.path().join("check.o");
        crate::native::host::run_tool(
            "x86_64-w64-mingw32-gcc",
            &[
                "-std=gnu17",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-c",
                source.to_str().unwrap_or_default(),
                "-o",
                object.to_str().unwrap_or_default(),
            ],
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "Requires Windows, MSVC cl on PATH (vcvars), TAC_WEBVIEW2_SDK and an installed WebView2 Runtime; runs the startup check against the installed, an old and a missing runtime, not a WebView2 page"]
    fn the_webview2_check_refuses_an_old_or_missing_runtime_with_the_install_message() {
        use crate::native::windows::WindowsHostGenerator;
        use crate::native::windows_architecture::WindowsArchitecture;
        use crate::native::windows_compiler_plan::WindowsCompilerPlan;
        use std::process::Command;

        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let sdk = std::path::PathBuf::from(
            std::env::var_os("TAC_WEBVIEW2_SDK").unwrap_or_else(|| panic!("set TAC_WEBVIEW2_SDK")),
        );
        let missing = root.path().join("no-runtime");
        std::fs::create_dir_all(&missing).unwrap_or_else(|error| panic!("{error}"));
        let mut application =
            NativeApplication::discover(root.path()).unwrap_or_else(|failure| panic!("{failure}"));
        let installed = WindowsWebEngineCheck::source(&application);
        application.minimum_system = application
            .minimum_system
            .with_floor(NativeTarget::Windows, SystemVersion::new(9999, 0));
        let old = WindowsWebEngineCheck::source(&application);
        for (name, check, folder, expected) in [
            ("current", &installed, None, "ready: yes\n"),
            (
                "old",
                &old,
                None,
                "ready: no\nmessage: Windows Host Check needs the Microsoft Edge WebView2 Runtime 9999.0 or newer.",
            ),
            (
                "missing",
                &installed,
                Some(&missing),
                "ready: no\nmessage: Windows Host Check needs the Microsoft Edge WebView2 Runtime 86.0 or newer.",
            ),
        ] {
            let harness = format!(
                "{}{}{}{}",
                include_str!("windows_web_engine_check_fixture.c"),
                "\nstatic int tachyon_capture_message(HWND owner, LPCWSTR text, LPCWSTR caption, UINT type) {\n  (void)owner;\n  (void)caption;\n  (void)type;\n  wprintf(L\"message: %ls\\n\", text);\n  return IDOK;\n}\n#define MessageBoxW tachyon_capture_message\n",
                check,
                "\nint WINAPI wWinMain(HINSTANCE instance, HINSTANCE previous, PWSTR command, int show) {\n  (void)instance;\n  (void)previous;\n  (void)command;\n  (void)show;\n  bool ready = tachyon_web_engine_ready();\n  wprintf(L\"ready: %ls\\n\", ready ? L\"yes\" : L\"no\");\n  if (!ready) tachyon_show_web_engine_message();\n  return 0;\n}\n"
            );
            let source = root.path().join(format!("{name}.c"));
            let program = root.path().join(format!("{name}.exe"));
            std::fs::write(&source, harness).unwrap_or_else(|error| panic!("{error}"));
            let plan = WindowsCompilerPlan {
                sdk: &sdk,
                source: source.to_str().unwrap_or_default(),
                executable: program.to_str().unwrap_or_default(),
                resource: None,
                architecture: WindowsArchitecture::build_machine(),
            };
            let compiled = Command::new("cl")
                .current_dir(root.path())
                .args(plan.arguments(&WindowsHostGenerator::LINKER_ARGUMENTS))
                .output()
                .unwrap_or_else(|error| panic!("{error}"));
            assert!(
                compiled.status.success(),
                "{}",
                String::from_utf8_lossy(&compiled.stdout)
            );
            let mut run = Command::new(&program);
            run.env_remove("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER");
            if let Some(folder) = folder {
                run.env("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", folder);
            }
            let output = run.output().unwrap_or_else(|error| panic!("{error}"));
            let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
            println!("{name}: {stdout}");
            assert!(output.status.success(), "{name}: {stdout}");
            assert!(stdout.starts_with(expected), "{name}: {stdout}");
        }
    }
}
