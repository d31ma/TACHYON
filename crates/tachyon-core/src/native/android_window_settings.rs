use super::config::WindowConfiguration;
use super::host::write;
use super::native_theme_color::NativeThemeColor;
use crate::Failure;
use std::path::Path;

pub(super) struct AndroidWindowSettings;

impl AndroidWindowSettings {
    const JAVA: &'static str = include_str!("android_window_settings.java");
    pub(super) const THEME: &'static str = "@style/TachyonTheme";
    const BASE_THEME: &'static str = "@android:style/Theme.Material.Light.NoActionBar";

    pub(super) fn java(window: &WindowConfiguration) -> String {
        let bar = window.theme_color.or(window.background_color);
        super::placeholder_fill::PlaceholderFill::once(
            Self::JAVA,
            &[
                (
                    "__WINDOW_INTEGRATED__",
                    &window.title_bar.is_integrated().to_string(),
                ),
                (
                    "__WINDOW_FULLSCREEN__",
                    &window.display.starts_fullscreen().to_string(),
                ),
                (
                    "__WINDOW_HAS_THEME_COLOR__",
                    &window.theme_color.is_some().to_string(),
                ),
                (
                    "__WINDOW_THEME_COLOR__",
                    &Self::java_color(window.theme_color),
                ),
                (
                    "__WINDOW_HAS_BACKGROUND_COLOR__",
                    &window.background_color.is_some().to_string(),
                ),
                (
                    "__WINDOW_BACKGROUND_COLOR__",
                    &Self::java_color(window.background_color),
                ),
                (
                    "__WINDOW_LIGHT_BARS__",
                    &bar.is_some_and(NativeThemeColor::is_light).to_string(),
                ),
            ],
        )
    }

    pub(super) fn activity_attributes(window: &WindowConfiguration) -> String {
        window
            .orientation
            .android_screen_orientation()
            .map_or_else(String::new, |orientation| {
                format!("\n        android:screenOrientation=\"{orientation}\"")
            })
    }

    pub(super) fn write_themes(
        project: &Path,
        window: &WindowConfiguration,
    ) -> Result<(), Failure> {
        let resources = project.join("app/src/main/res");
        write(
            &resources.join("values/themes.xml"),
            Self::themes(window).as_bytes(),
        )?;
        let Some(background) = window.background_color else {
            return Ok(());
        };
        write(
            &resources.join("values-v31/themes.xml"),
            Self::splash_theme(background).as_bytes(),
        )
    }

    fn themes(window: &WindowConfiguration) -> String {
        let mut items = String::new();
        if let Some(background) = window.background_color {
            items.push_str(&Self::item("android:windowBackground", background));
        }
        if let Some(theme) = window.theme_color {
            items.push_str(&Self::item("android:statusBarColor", theme));
            items.push_str(&Self::item("android:navigationBarColor", theme));
        }
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n  <style name=\"TachyonBase\" parent=\"{}\">\n{items}  </style>\n  <style name=\"TachyonTheme\" parent=\"@style/TachyonBase\"/>\n</resources>\n",
            Self::BASE_THEME
        )
    }

    fn splash_theme(background: NativeThemeColor) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n  <style name=\"TachyonTheme\" parent=\"@style/TachyonBase\">\n{}  </style>\n</resources>\n",
            Self::item("android:windowSplashScreenBackground", background)
        )
    }

    fn item(name: &str, color: NativeThemeColor) -> String {
        format!(
            "    <item name=\"{name}\">#{:02x}{:02x}{:02x}</item>\n",
            color.red, color.green, color.blue
        )
    }

    fn java_color(color: Option<NativeThemeColor>) -> String {
        color.map_or_else(
            || String::from("0"),
            |color| format!("0xff{:02x}{:02x}{:02x}", color.red, color.green, color.blue),
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{AndroidWindowSettings, WindowConfiguration};
    use crate::native::native_display::NativeDisplay;
    use crate::native::native_orientation::NativeOrientation;
    use crate::native::native_theme_color::NativeThemeColor;
    use crate::native::native_title_bar::NativeTitleBar;

    fn color(red: u8, green: u8, blue: u8) -> NativeThemeColor {
        NativeThemeColor { red, green, blue }
    }

    #[test]
    fn a_default_window_keeps_the_bars_and_turns_with_the_device() {
        let window = WindowConfiguration::default();
        let java = AndroidWindowSettings::java(&window);
        assert!(java.contains("WINDOW_INTEGRATED = false;"));
        assert!(java.contains("WINDOW_FULLSCREEN = false;"));
        assert!(java.contains("WINDOW_HAS_THEME_COLOR = false;"));
        assert!(java.contains("WINDOW_THEME_COLOR = 0;"));
        assert!(!java.contains("__WINDOW_"));
        assert_eq!(AndroidWindowSettings::activity_attributes(&window), "");
        let themes = AndroidWindowSettings::themes(&window);
        assert!(themes.contains("<style name=\"TachyonTheme\" parent=\"@style/TachyonBase\"/>"));
        assert!(!themes.contains("<item"));
    }

    #[test]
    fn the_manifest_colors_reach_the_bars_the_window_and_the_splash_screen() {
        let window = WindowConfiguration {
            theme_color: Some(color(0x0f, 0x6c, 0xbd)),
            background_color: Some(color(0xff, 0xee, 0xdd)),
            ..WindowConfiguration::default()
        };
        let java = AndroidWindowSettings::java(&window);
        assert!(java.contains("WINDOW_THEME_COLOR = 0xff0f6cbd;"));
        assert!(java.contains("WINDOW_BACKGROUND_COLOR = 0xffffeedd;"));
        assert!(java.contains("WINDOW_LIGHT_BARS = false;"));
        let themes = AndroidWindowSettings::themes(&window);
        assert!(themes.contains("<item name=\"android:windowBackground\">#ffeedd</item>"));
        assert!(themes.contains("<item name=\"android:statusBarColor\">#0f6cbd</item>"));
        let project = tempfile::tempdir().expect("project");
        AndroidWindowSettings::write_themes(project.path(), &window).expect("themes");
        let splash = std::fs::read_to_string(
            project
                .path()
                .join("app/src/main/res/values-v31/themes.xml"),
        )
        .expect("splash");
        assert!(splash.contains("android:windowSplashScreenBackground\">#ffeedd"));
    }

    #[test]
    fn fullscreen_integrated_and_a_locked_orientation_reach_the_activity() {
        let window = WindowConfiguration {
            title_bar: NativeTitleBar::Integrated,
            display: NativeDisplay::Fullscreen,
            orientation: NativeOrientation::Landscape,
            background_color: Some(color(0xff, 0xff, 0xff)),
            ..WindowConfiguration::default()
        };
        let java = AndroidWindowSettings::java(&window);
        assert!(java.contains("WINDOW_INTEGRATED = true;"));
        assert!(java.contains("WINDOW_FULLSCREEN = true;"));
        assert!(java.contains("WINDOW_LIGHT_BARS = true;"));
        assert_eq!(
            AndroidWindowSettings::activity_attributes(&window),
            "\n        android:screenOrientation=\"userLandscape\""
        );
    }
}
