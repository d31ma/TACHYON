use super::config::WindowConfiguration;
use super::native_theme_color::NativeThemeColor;

pub(super) struct WindowsWindowBackground;

impl WindowsWindowBackground {
    const WINDOW: &'static str = include_str!("windows_window_background.c");
    const WEB_VIEW: &'static str = include_str!("windows_web_view_background.c");
    const NO_BRUSH: &'static str = "NULL";
    const NO_WEB_VIEW_COLOR: &'static str = "  (void)controller;\n";

    pub(super) fn window_source(window: &WindowConfiguration) -> String {
        let color = window.background_color;
        let brush = color
            .filter(|_| !window.title_bar.is_integrated())
            .map_or_else(|| String::from(Self::NO_BRUSH), Self::brush);
        Self::WINDOW
            .replace("__WINDOW_BACKGROUND_BRUSH__", &brush)
            .replace(
                "__WINDOW_BACKGROUND_ENVIRONMENT__",
                &color.map(Self::environment).unwrap_or_default(),
            )
    }

    pub(super) fn web_view_source(window: &WindowConfiguration) -> String {
        let body = window.background_color.map_or_else(
            || String::from(Self::NO_WEB_VIEW_COLOR),
            Self::web_view_color,
        );
        Self::WEB_VIEW.replace("__WEB_VIEW_COLOR__", &body)
    }

    fn brush(color: NativeThemeColor) -> String {
        format!(
            "CreateSolidBrush(RGB({}, {}, {}))",
            color.red, color.green, color.blue
        )
    }

    fn environment(color: NativeThemeColor) -> String {
        format!(
            "  SetEnvironmentVariableW(L\"WEBVIEW2_DEFAULT_BACKGROUND_COLOR\", L\"FF{:02X}{:02X}{:02X}\");\n",
            color.red, color.green, color.blue
        )
    }

    fn web_view_color(color: NativeThemeColor) -> String {
        format!(
            "  ICoreWebView2Controller2 *colored = NULL;\n  \
             if (FAILED(ICoreWebView2Controller_QueryInterface(controller, &IID_ICoreWebView2Controller2, (void **)&colored)) || colored == NULL) return;\n  \
             COREWEBVIEW2_COLOR background = {{255, {}, {}, {}}};\n  \
             ICoreWebView2Controller2_put_DefaultBackgroundColor(colored, background);\n  \
             ICoreWebView2Controller2_Release(colored);\n",
            color.red, color.green, color.blue
        )
    }
}

#[cfg(test)]
mod tests {
    use super::WindowsWindowBackground;
    use crate::native::config::WindowConfiguration;
    use crate::native::native_theme_color::NativeThemeColor;
    use crate::native::native_title_bar::NativeTitleBar;

    fn colored(title_bar: NativeTitleBar) -> WindowConfiguration {
        WindowConfiguration {
            title_bar,
            background_color: Some(NativeThemeColor {
                red: 0x1a,
                green: 0x23,
                blue: 0x7e,
            }),
            ..WindowConfiguration::default()
        }
    }

    #[test]
    fn an_undeclared_background_keeps_the_class_brush_and_web_view_defaults() {
        let window = WindowConfiguration::default();
        let source = WindowsWindowBackground::window_source(&window);
        assert!(source.contains("return NULL;"));
        assert!(!source.contains("SetEnvironmentVariableW"));
        let web_view = WindowsWindowBackground::web_view_source(&window);
        assert!(web_view.contains("(void)controller;"));
        assert!(!web_view.contains("put_DefaultBackgroundColor"));
        assert!(!format!("{source}{web_view}").contains("__"));
    }

    #[test]
    fn background_color_paints_the_window_and_the_web_view_before_the_first_page() {
        let window = colored(NativeTitleBar::Standard);
        let source = WindowsWindowBackground::window_source(&window);
        assert!(source.contains("return CreateSolidBrush(RGB(26, 35, 126));"));
        assert!(source.contains(
            "SetEnvironmentVariableW(L\"WEBVIEW2_DEFAULT_BACKGROUND_COLOR\", L\"FF1A237E\");"
        ));
        let web_view = WindowsWindowBackground::web_view_source(&window);
        assert!(web_view.contains("&IID_ICoreWebView2Controller2"));
        assert!(web_view.contains("COREWEBVIEW2_COLOR background = {255, 26, 35, 126};"));
        assert!(
            web_view.contains(
                "ICoreWebView2Controller2_put_DefaultBackgroundColor(colored, background);"
            )
        );
    }

    #[test]
    fn an_integrated_title_bar_keeps_its_glass_frame_but_colors_the_web_view() {
        let window = colored(NativeTitleBar::Integrated);
        let source = WindowsWindowBackground::window_source(&window);
        assert!(source.contains("return NULL;"));
        assert!(source.contains("L\"FF1A237E\""));
        assert!(
            WindowsWindowBackground::web_view_source(&window)
                .contains("put_DefaultBackgroundColor")
        );
    }
}
