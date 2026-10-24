#![allow(clippy::expect_used)]

use super::config::WindowConfiguration;
use super::native_display::NativeDisplay;
use super::native_theme_color::NativeThemeColor;

pub(super) struct LinuxWindowAppearanceTests;

impl LinuxWindowAppearanceTests {
    const SERVICE: &'static str = r"
#[Service]
#[derive(Default)]
struct AppearanceService {
    window_controls: WindowControls,
}

impl AppearanceService {
    fn granted(&self, control: String) -> bool {
        self.window_controls.granted(&control).unwrap_or(false)
    }
}
";

    fn themed_window() -> WindowConfiguration {
        WindowConfiguration {
            width: 900,
            height: 640,
            min_width: Some(720),
            min_height: Some(520),
            controls: vec![String::from("resize")],
            theme_color: Some(NativeThemeColor {
                red: 0x1a,
                green: 0x23,
                blue: 0x7e,
            }),
            background_color: Some(NativeThemeColor {
                red: 0xfd,
                green: 0xf6,
                blue: 0xe3,
            }),
            display: NativeDisplay::Standalone,
            ..WindowConfiguration::default()
        }
    }

    fn fullscreen_window() -> WindowConfiguration {
        WindowConfiguration {
            display: NativeDisplay::Fullscreen,
            ..WindowConfiguration::default()
        }
    }

    fn fixture(fullscreen: bool) -> String {
        let (flag, steps) = if fullscreen { (1, 2) } else { (0, 6) };
        format!(
            "#define TACHYON_APPEARANCE_FULLSCREEN {flag}\n#define TACHYON_APPEARANCE_STEPS {steps}\n{}",
            include_str!("linux_window_appearance_fixture.c")
        )
    }

    #[cfg(target_os = "linux")]
    pub(super) fn stage(root: &std::path::Path) -> std::path::PathBuf {
        let source = root.join("client/services/AppearanceService.rs");
        std::fs::create_dir_all(source.parent().expect("parent")).expect("directory");
        std::fs::write(&source, Self::SERVICE).expect("service source");
        let input = super::registry::NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: crate::project::NativeCompanion::Rust,
            source,
            service: String::from("appearance"),
            class: String::from("AppearanceService"),
            web_assembly: false,
            application_provider: false,
        };
        super::rust::stage(&[input], root, "org.tachyon.window-appearance")
            .expect("staged Rust services")
            .expect("native Rust source")
    }

    #[cfg(target_os = "linux")]
    async fn run(window: WindowConfiguration, fullscreen: bool) -> String {
        let root = tempfile::tempdir().expect("fixture");
        let source = Self::stage(root.path());
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.window = window;
        let executable = super::linux_host_tests::LinuxHostTests::compile_application(
            root.path(),
            &source,
            &application,
            &Self::fixture(fullscreen),
        )
        .await;
        super::host::run_tool(executable.to_str().expect("binary"), &[])
            .await
            .expect("actual generated Linux host window appearance")
    }
}

#[test]
fn the_appearance_fixture_checks_each_declared_member_of_both_windows() {
    let themed = super::linux::c_source(
        &super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        ),
        &super::linux_host_tests::LinuxHostTests::index(),
    );
    assert!(themed.contains("tachyon_window_appearance(GTK_WINDOW(window), GTK_WIDGET(view));"));
    let source = super::linux_window_appearance::LinuxWindowAppearance::source(
        &LinuxWindowAppearanceTests::themed_window(),
    );
    assert!(source.contains("gtk_window_set_default_size(window, 900, 640);"));
    assert!(source.contains("gtk_widget_set_size_request(content, 720, 520);"));
    assert!(source.contains("background: #1a237e; color: #ffffff;"));
    assert!(source.contains("gdk_rgba_parse(&background, \"#fdf6e3\");"));
    assert!(!source.contains("gtk_window_fullscreen"));
    let fullscreen = super::linux_window_appearance::LinuxWindowAppearance::source(
        &LinuxWindowAppearanceTests::fullscreen_window(),
    );
    assert!(fullscreen.contains("gtk_window_fullscreen(window);"));
    assert!(!fullscreen.contains("gtk_header_bar_new"));
    assert!(!fullscreen.contains("webkit_web_view_set_background_color"));
    for fullscreen in [false, true] {
        let fixture = LinuxWindowAppearanceTests::fixture(fullscreen);
        assert!(fixture.contains("tachyon_capability_call(\"window.resize\""));
        assert!(fixture.contains("int main(int argc, char **argv)"));
    }
    assert!(LinuxWindowAppearanceTests::SERVICE.contains("struct AppearanceService"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files and an X11 display with a window manager (xfwm4 under Xvfb); run explicitly on the Linux lane"]
async fn linux_generated_host_opens_the_declared_size_minimum_and_theme_color() {
    let output =
        LinuxWindowAppearanceTests::run(LinuxWindowAppearanceTests::themed_window(), false).await;
    assert!(
        output.contains("linux window appearance passed"),
        "{output}"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files and an X11 display with a window manager (xfwm4 under Xvfb); run explicitly on the Linux lane"]
async fn linux_generated_host_starts_fullscreen_when_the_manifest_display_is_fullscreen() {
    let output =
        LinuxWindowAppearanceTests::run(LinuxWindowAppearanceTests::fullscreen_window(), true)
            .await;
    assert!(
        output.contains("linux window appearance passed"),
        "{output}"
    );
}
