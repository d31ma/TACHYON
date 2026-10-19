#![allow(clippy::expect_used)]

use super::config::WindowConfiguration;
use super::native_title_bar::NativeTitleBar;
use super::native_title_bar_button_inset::NativeTitleBarButtonInset;

struct LinuxTitleBarTests;

impl LinuxTitleBarTests {
    fn integrated_window() -> WindowConfiguration {
        WindowConfiguration {
            controls: vec![String::from("title")],
            title_bar: NativeTitleBar::Integrated,
            ..WindowConfiguration::default()
        }
    }

    const DECLARED_INSET: NativeTitleBarButtonInset = NativeTitleBarButtonInset {
        leading: 20,
        top: 14,
    };

    fn application() -> super::config::NativeApplication {
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.window = Self::integrated_window();
        application
    }

    fn declared_inset_fixture() -> String {
        format!(
            "#define TACHYON_BAR_DECLARED_TOP {}\n#define TACHYON_BAR_DECLARED_SIDE {}\n{}",
            Self::DECLARED_INSET.top,
            Self::DECLARED_INSET.leading,
            include_str!("linux_title_bar_fixture.c")
        )
    }

    #[cfg(target_os = "linux")]
    async fn run(application: &super::config::NativeApplication, fixture: &str) -> String {
        let root = tempfile::tempdir().expect("fixture");
        let source =
            super::linux_window_appearance_tests::LinuxWindowAppearanceTests::stage(root.path());
        let executable = super::linux_host_tests::LinuxHostTests::compile_application(
            root.path(),
            &source,
            application,
            fixture,
        )
        .await;
        super::host::run_tool(executable.to_str().expect("binary"), &[])
            .await
            .expect("actual generated Linux host title bar")
    }

    #[cfg(target_os = "linux")]
    fn passed() -> &'static str {
        if std::env::var("GDK_BACKEND").is_ok_and(|backend| backend == "wayland") {
            return "linux title bar passed on Wayland";
        }
        "linux title bar passed on X11"
    }
}

#[test]
fn the_linux_host_wraps_an_integrated_page_and_routes_its_drag_regions() {
    let source = super::linux::c_source(
        &LinuxTitleBarTests::application(),
        &super::linux_host_tests::LinuxHostTests::index(),
    );
    assert!(source.contains(
        "gtk_window_set_child(GTK_WINDOW(window), tachyon_title_bar_content(GTK_WINDOW(window), view, manager));"
    ));
    assert!(source.contains("answer = tachyon_title_bar_regions(payload);"));
    assert!(
        source.contains("gtk_window_set_title(g_native_window, tachyon_title_bar_title(title));")
    );
    assert!(source.contains("TACHYON_TITLE_BAR_INTEGRATED = TRUE;"));
    assert!(!source.contains("__NATIVE_TITLE_BAR__"));
    let fixture = include_str!("linux_title_bar_fixture.c");
    assert!(fixture.contains("XTestFakeButtonEvent"));
    assert!(fixture.contains("int main(int argc, char **argv)"));
}

#[test]
fn an_integrated_title_bar_leaves_no_room_for_the_theme_color_header_bar() {
    let window = WindowConfiguration {
        theme_color: Some(super::native_theme_color::NativeThemeColor {
            red: 0x1a,
            green: 0x23,
            blue: 0x7e,
        }),
        ..LinuxTitleBarTests::integrated_window()
    };
    let source = super::linux_window_appearance::LinuxWindowAppearance::source(&window);
    assert!(!source.contains("gtk_header_bar_new"));
}

#[test]
fn a_declared_inset_reaches_the_title_bar_fixture() {
    let fixture = LinuxTitleBarTests::declared_inset_fixture();
    assert!(fixture.starts_with(
        "#define TACHYON_BAR_DECLARED_TOP 14\n#define TACHYON_BAR_DECLARED_SIDE 20\n"
    ));
    assert!(fixture.contains("NotifyPointerMotionRelative"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, and either libXtst and an X11 display with a window manager (xfwm4 under Xvfb) or a headless mutter Wayland session (scripts/linux/compositor-test.sh); run explicitly on the Linux lane"]
async fn linux_generated_host_integrates_the_title_bar_with_insets_and_drag_regions() {
    let output = LinuxTitleBarTests::run(
        &LinuxTitleBarTests::application(),
        include_str!("linux_title_bar_fixture.c"),
    )
    .await;
    assert!(output.contains(LinuxTitleBarTests::passed()), "{output}");
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, and either libXtst and an X11 display with a window manager (xfwm4 under Xvfb) or a headless mutter Wayland session (scripts/linux/compositor-test.sh); run explicitly on the Linux lane"]
async fn linux_generated_host_places_the_window_buttons_at_the_declared_inset() {
    let mut application = LinuxTitleBarTests::application();
    application.window.title_bar_button_inset = Some(LinuxTitleBarTests::DECLARED_INSET);
    let output =
        LinuxTitleBarTests::run(&application, &LinuxTitleBarTests::declared_inset_fixture()).await;
    assert!(output.contains(LinuxTitleBarTests::passed()), "{output}");
}
