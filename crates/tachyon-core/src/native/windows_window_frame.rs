use super::config::WindowConfiguration;
use super::native_theme_color::NativeThemeColor;

pub(super) struct WindowsWindowFrame;

impl WindowsWindowFrame {
    const LIMITS_MESSAGES: [&str; 2] = [
        "",
        "    case WM_GETMINMAXINFO:\n      tachyon_window_limits(window, (MINMAXINFO *)lparam);\n      return 0;\n",
    ];
    const WIDTH_LIMIT: &str =
        "  if (minimum.cx > limits->ptMinTrackSize.x) limits->ptMinTrackSize.x = minimum.cx;\n";
    const HEIGHT_LIMIT: &str =
        "  if (minimum.cy > limits->ptMinTrackSize.y) limits->ptMinTrackSize.y = minimum.cy;\n";
    const FULLSCREEN_OPENING: [&str; 2] = [
        "",
        "  if (tachyon_window_toggle_fullscreen(window) && show == SW_SHOWMAXIMIZED) show = SW_SHOWNORMAL;\n",
    ];
    const CAPTION_TEXT: [u8; 2] = [255, 0];

    pub(super) fn source(window: &WindowConfiguration) -> String {
        include_str!("windows_window_frame.c")
            .replace("__WINDOW_LIMITS__", &Self::limits(window))
            .replace("__WINDOW_WIDTH__", &window.width.to_string())
            .replace("__WINDOW_HEIGHT__", &window.height.to_string())
            .replace(
                "__WINDOW_TOP__",
                super::windows_title_bar::WindowsTitleBar::window_top(window),
            )
            .replace("__WINDOW_OPENING__", &Self::opening(window))
    }

    pub(super) fn messages(window: &WindowConfiguration) -> &'static str {
        Self::LIMITS_MESSAGES[usize::from(Self::has_minimum(window))]
    }

    fn has_minimum(window: &WindowConfiguration) -> bool {
        [window.min_width.is_some(), window.min_height.is_some()].contains(&true)
    }

    fn limits(window: &WindowConfiguration) -> String {
        let width = window.min_width.map_or("", |_| Self::WIDTH_LIMIT);
        let height = window.min_height.map_or("", |_| Self::HEIGHT_LIMIT);
        let body = format!(
            "\nstatic void tachyon_window_limits(HWND window, MINMAXINFO *limits) {{\n  SIZE minimum = tachyon_window_outer(window, {}, {});\n{width}{height}}}\n",
            window.min_width.unwrap_or(0),
            window.min_height.unwrap_or(0)
        );
        if Self::has_minimum(window) {
            body
        } else {
            String::new()
        }
    }

    fn opening(window: &WindowConfiguration) -> String {
        let caption = window.theme_color.map(Self::caption).unwrap_or_default();
        let fullscreen = Self::FULLSCREEN_OPENING[usize::from(window.display.starts_fullscreen())];
        format!("{caption}{fullscreen}")
    }

    fn caption(color: NativeThemeColor) -> String {
        let text = Self::CAPTION_TEXT[usize::from(color.is_light())];
        format!(
            "  COLORREF caption = RGB({}, {}, {}), caption_text = RGB({text}, {text}, {text});\n  DwmSetWindowAttribute(window, TACHYON_CAPTION_COLOR_ATTRIBUTE, &caption, sizeof(caption));\n  DwmSetWindowAttribute(window, TACHYON_CAPTION_TEXT_ATTRIBUTE, &caption_text, sizeof(caption_text));\n",
            color.red, color.green, color.blue
        )
    }
}
