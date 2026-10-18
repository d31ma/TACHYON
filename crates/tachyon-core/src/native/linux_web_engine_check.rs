use super::config::NativeApplication;
use tachyon_contracts::NativeTarget;

pub(super) struct LinuxWebEngineCheck;

impl LinuxWebEngineCheck {
    const SOURCE: &'static str = include_str!("linux_web_engine_check.c");

    pub(super) fn source(application: &NativeApplication) -> String {
        let floor = application.minimum_system.floor(NativeTarget::Linux);
        Self::SOURCE
            .replace("__WEBKITGTK_MAJOR__", &floor.major().to_string())
            .replace("__WEBKITGTK_MINOR__", &floor.minor().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::LinuxWebEngineCheck;
    use crate::native::config::NativeApplication;
    use crate::native::system_version::SystemVersion;
    use tachyon_contracts::NativeTarget;

    #[test]
    fn the_linux_host_checks_the_webkitgtk_floor_before_it_shows_a_page() {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let mut application =
            NativeApplication::discover(root.path()).unwrap_or_else(|failure| panic!("{failure}"));
        let source = LinuxWebEngineCheck::source(&application);
        assert!(source.contains("#define TACHYON_WEBKITGTK_MAJOR 2u"));
        assert!(source.contains("#define TACHYON_WEBKITGTK_MINOR 40u"));
        application.minimum_system = application
            .minimum_system
            .with_floor(NativeTarget::Linux, SystemVersion::new(2, 46));
        let raised = LinuxWebEngineCheck::source(&application);
        assert!(raised.contains("#define TACHYON_WEBKITGTK_MINOR 46u"));
        assert!(raised.contains("webkit_get_minor_version()"));
        assert!(!raised.contains("__WEBKITGTK_"));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    #[ignore = "Requires GTK 4, WebKitGTK 6.0 development files and a display; run explicitly on the Linux lane"]
    async fn an_old_webkitgtk_shows_a_message_instead_of_a_blank_window() {
        use crate::native::linux_host_tests::LinuxHostTests;
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let source = crate::native::native_menu_action_tests::MenuActionFixture::stage(root.path());
        let mut application = LinuxHostTests::application(LinuxHostTests::menus());
        application.minimum_system = application
            .minimum_system
            .with_floor(NativeTarget::Linux, SystemVersion::new(99, 0));
        let executable = LinuxHostTests::compile_application(
            root.path(),
            &source,
            &application,
            include_str!("linux_web_engine_check_fixture.c"),
        )
        .await;
        let output = crate::native::host::run_tool(
            executable.to_str().unwrap_or_else(|| panic!("binary path")),
            &[],
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
        assert!(
            output.contains(
                "old web engine message shown without a page: Linux Host Check needs WebKitGTK 99.0 or newer"
            ),
            "{output}"
        );
    }
}
