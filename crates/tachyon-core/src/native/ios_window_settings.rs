use super::config::WindowConfiguration;
use super::native_theme_color::NativeThemeColor;

pub(super) struct IosWindowSettings;

impl IosWindowSettings {
    const SWIFT: &'static str = include_str!("IosWindowSettings.swift");
    const INSETS_CASE: &'static str = "        case \"window.titleBarInsets\":\n            return IosWindowSettings.insets(webView?.superview)\n";
    const FULL_SCREEN_KEYS: &'static str = "\n<key>UIRequiresFullScreen</key><true/>";
    const HIDDEN_STATUS_BAR_KEYS: &'static str = "\n<key>UIStatusBarHidden</key><true/>";

    pub(super) fn swift(window: &WindowConfiguration) -> String {
        let bar = window.theme_color.or(window.background_color);
        super::placeholder_fill::PlaceholderFill::once(
            Self::SWIFT,
            &[
                (
                    "__WINDOW_INTEGRATED__",
                    &window.title_bar.is_integrated().to_string(),
                ),
                (
                    "__WINDOW_FULLSCREEN__",
                    &window.display.starts_fullscreen().to_string(),
                ),
                ("__WINDOW_THEME_COLOR__", &Self::color(window.theme_color)),
                (
                    "__WINDOW_BACKGROUND_COLOR__",
                    &Self::color(window.background_color),
                ),
                ("__WINDOW_STATUS_BAR_STYLE__", Self::status_bar_style(bar)),
            ],
        )
    }

    pub(super) fn bridge_case(window: &WindowConfiguration) -> &'static str {
        ["", Self::INSETS_CASE][usize::from(window.title_bar.is_integrated())]
    }

    pub(super) fn plist_entries(window: &WindowConfiguration) -> String {
        let orientations = window
            .orientation
            .ios_orientations()
            .iter()
            .map(|name| ["<string>", name, "</string>"].concat())
            .collect::<String>();
        let full_screen = ["", Self::FULL_SCREEN_KEYS][usize::from(window.orientation.is_locked())];
        let hidden =
            ["", Self::HIDDEN_STATUS_BAR_KEYS][usize::from(window.display.starts_fullscreen())];
        format!(
            "{full_screen}{hidden}\n<key>UISupportedInterfaceOrientations</key><array>{orientations}</array>\
             \n<key>UISupportedInterfaceOrientations~ipad</key><array>{orientations}</array>"
        )
    }

    fn color(color: Option<NativeThemeColor>) -> String {
        color.map_or_else(
            || String::from("nil"),
            |color| {
                format!(
                    "UIColor(red: {} / 255, green: {} / 255, blue: {} / 255, alpha: 1)",
                    color.red, color.green, color.blue
                )
            },
        )
    }

    fn status_bar_style(bar: Option<NativeThemeColor>) -> &'static str {
        match bar.map(NativeThemeColor::is_light) {
            Some(true) => ".darkContent",
            Some(false) => ".lightContent",
            None => ".default",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IosWindowSettings, WindowConfiguration};
    use crate::native::native_display::NativeDisplay;
    use crate::native::native_orientation::NativeOrientation;
    use crate::native::native_theme_color::NativeThemeColor;
    use crate::native::native_title_bar::NativeTitleBar;

    fn color(red: u8, green: u8, blue: u8) -> NativeThemeColor {
        NativeThemeColor { red, green, blue }
    }

    #[test]
    fn a_default_window_keeps_the_status_bar_and_turns_with_the_device() {
        let window = WindowConfiguration::default();
        let source = IosWindowSettings::swift(&window);
        assert!(source.contains("static let integrated = false"));
        assert!(source.contains("static let fullscreen = false"));
        assert!(source.contains("static let themeColor: UIColor? = nil"));
        assert!(source.contains("static let statusBarStyle: UIStatusBarStyle = .default"));
        assert!(!source.contains("__WINDOW_"));
        assert_eq!(IosWindowSettings::bridge_case(&window), "");
        let plist = IosWindowSettings::plist_entries(&window);
        assert_eq!(
            plist
                .matches("UIInterfaceOrientationPortraitUpsideDown")
                .count(),
            2
        );
        assert!(!plist.contains("UIRequiresFullScreen"));
        assert!(!plist.contains("UIStatusBarHidden"));
    }

    #[test]
    fn the_manifest_colors_the_status_bar_and_picks_readable_icons() {
        let dark = WindowConfiguration {
            theme_color: Some(color(0x0f, 0x6c, 0xbd)),
            background_color: Some(color(0xff, 0xff, 0xff)),
            ..WindowConfiguration::default()
        };
        let source = IosWindowSettings::swift(&dark);
        assert!(source.contains(
            "static let themeColor: UIColor? = UIColor(red: 15 / 255, green: 108 / 255, blue: 189 / 255, alpha: 1)"
        ));
        assert!(source.contains("static let backgroundColor: UIColor? = UIColor(red: 255 / 255"));
        assert!(source.contains("statusBarStyle: UIStatusBarStyle = .lightContent"));
        let light_background = WindowConfiguration {
            background_color: Some(color(0xff, 0xff, 0xff)),
            ..WindowConfiguration::default()
        };
        assert!(
            IosWindowSettings::swift(&light_background)
                .contains("statusBarStyle: UIStatusBarStyle = .darkContent")
        );
    }

    #[test]
    fn an_integrated_title_bar_draws_under_the_status_bar_and_answers_the_insets() {
        let window = WindowConfiguration {
            title_bar: NativeTitleBar::Integrated,
            ..WindowConfiguration::default()
        };
        assert!(IosWindowSettings::swift(&window).contains("static let integrated = true"));
        assert!(IosWindowSettings::bridge_case(&window).contains("case \"window.titleBarInsets\""));
    }

    #[test]
    fn fullscreen_and_a_locked_orientation_reach_the_plist() {
        let window = WindowConfiguration {
            display: NativeDisplay::Fullscreen,
            orientation: NativeOrientation::Landscape,
            ..WindowConfiguration::default()
        };
        let plist = IosWindowSettings::plist_entries(&window);
        assert!(plist.contains("<key>UIStatusBarHidden</key><true/>"));
        assert!(plist.contains("<key>UIRequiresFullScreen</key><true/>"));
        assert!(plist.contains("<key>UISupportedInterfaceOrientations</key><array><string>UIInterfaceOrientationLandscapeLeft</string><string>UIInterfaceOrientationLandscapeRight</string></array>"));
        assert!(!plist.contains("Portrait"));
        assert!(IosWindowSettings::swift(&window).contains("static let fullscreen = true"));
    }
}
