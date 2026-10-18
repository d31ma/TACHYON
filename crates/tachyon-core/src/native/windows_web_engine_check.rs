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
}
