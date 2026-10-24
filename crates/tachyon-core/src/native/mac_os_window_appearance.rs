use super::config::WindowConfiguration;
use super::native_theme_color::NativeThemeColor;

pub(super) struct MacOsWindowAppearance;

impl MacOsWindowAppearance {
    const SWIFT: &'static str = include_str!("MacOsWindowAppearance.swift");
    const TITLE_APPEARANCES: [&'static str; 2] = [".darkAqua", ".aqua"];

    pub(super) fn swift(window: &WindowConfiguration) -> String {
        let theme = window
            .theme_color
            .filter(|_| !window.title_bar.is_integrated());
        let title = theme.map_or("nil", |color| {
            Self::TITLE_APPEARANCES[usize::from(color.is_light())]
        });
        super::placeholder_fill::PlaceholderFill::once(
            Self::SWIFT,
            &[
                ("__WINDOW_THEME_COLOR__", &Self::color(theme)),
                (
                    "__WINDOW_BACKGROUND_COLOR__",
                    &Self::color(window.background_color),
                ),
                ("__WINDOW_TITLE_APPEARANCE__", title),
                (
                    "__WINDOW_FULLSCREEN__",
                    &window.display.starts_fullscreen().to_string(),
                ),
            ],
        )
    }

    fn color(color: Option<NativeThemeColor>) -> String {
        color.map_or_else(
            || String::from("nil"),
            |color| {
                format!(
                    "NSColor(srgbRed: {} / 255, green: {} / 255, blue: {} / 255, alpha: 1)",
                    color.red, color.green, color.blue
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::MacOsWindowAppearance;
    use crate::native::config::WindowConfiguration;
    use crate::native::native_display::NativeDisplay;
    use crate::native::native_theme_color::NativeThemeColor;
    use crate::native::native_title_bar::NativeTitleBar;

    fn color(red: u8, green: u8, blue: u8) -> NativeThemeColor {
        NativeThemeColor { red, green, blue }
    }

    #[test]
    fn an_undeclared_window_keeps_the_standard_title_bar_background_and_window() {
        let source = MacOsWindowAppearance::swift(&WindowConfiguration::default());
        assert!(source.contains("static let themeColor: NSColor? = nil"));
        assert!(source.contains("static let backgroundColor: NSColor? = nil"));
        assert!(source.contains("static let titleAppearance: NSAppearance.Name? = nil"));
        assert!(source.contains("static let fullscreen = false"));
        assert!(!source.contains("__WINDOW_"));
    }

    #[test]
    fn theme_color_colors_the_title_bar_with_readable_title_text() {
        let dark = WindowConfiguration {
            theme_color: Some(color(0x1a, 0x23, 0x7e)),
            background_color: Some(color(0xfd, 0xf6, 0xe3)),
            ..WindowConfiguration::default()
        };
        let source = MacOsWindowAppearance::swift(&dark);
        assert!(source.contains(
            "static let themeColor: NSColor? = NSColor(srgbRed: 26 / 255, green: 35 / 255, blue: 126 / 255, alpha: 1)"
        ));
        assert!(source.contains(
            "static let backgroundColor: NSColor? = NSColor(srgbRed: 253 / 255, green: 246 / 255, blue: 227 / 255, alpha: 1)"
        ));
        assert!(source.contains("static let titleAppearance: NSAppearance.Name? = .darkAqua"));
        let light = WindowConfiguration {
            theme_color: Some(color(0xff, 0xd7, 0x00)),
            ..WindowConfiguration::default()
        };
        assert!(
            MacOsWindowAppearance::swift(&light)
                .contains("static let titleAppearance: NSAppearance.Name? = .aqua")
        );
    }

    #[test]
    fn an_integrated_title_bar_leaves_theme_color_to_the_page_but_keeps_the_background() {
        let window = WindowConfiguration {
            title_bar: NativeTitleBar::Integrated,
            theme_color: Some(color(0x1a, 0x23, 0x7e)),
            background_color: Some(color(0, 0, 0)),
            display: NativeDisplay::Fullscreen,
            ..WindowConfiguration::default()
        };
        let source = MacOsWindowAppearance::swift(&window);
        assert!(source.contains("static let themeColor: NSColor? = nil"));
        assert!(source.contains("static let titleAppearance: NSAppearance.Name? = nil"));
        assert!(source.contains("static let backgroundColor: NSColor? = NSColor(srgbRed: 0 / 255"));
        assert!(source.contains("static let fullscreen = true"));
    }
}
