use super::config::WindowConfiguration;
use super::native_theme_color::NativeThemeColor;

pub(super) struct LinuxWindowAppearance;

impl LinuxWindowAppearance {
    const SOURCE: &'static str = include_str!("linux_window_appearance.c");
    const UNSET_EDGE: i64 = -1;
    const TEXT_COLORS: [&'static str; 2] = ["#ffffff", "#000000"];

    pub(super) fn source(window: &WindowConfiguration) -> String {
        Self::SOURCE
            .replace("__WINDOW_WIDTH__", &window.width.to_string())
            .replace("__WINDOW_HEIGHT__", &window.height.to_string())
            .replace("__WINDOW_MINIMUM_SIZE__", &Self::minimum_size(window))
            .replace("__WINDOW_THEME_COLOR__", &Self::theme_color(window))
            .replace("__WINDOW_DISPLAY__", Self::display(window))
    }

    fn minimum_size(window: &WindowConfiguration) -> String {
        if window.min_width.is_none() && window.min_height.is_none() {
            return String::new();
        }
        let width = window.min_width.map_or(Self::UNSET_EDGE, i64::from);
        let height = window.min_height.map_or(Self::UNSET_EDGE, i64::from);
        format!("gtk_widget_set_size_request(content, {width}, {height});")
    }

    fn theme_color(window: &WindowConfiguration) -> String {
        let Some(color) = window.theme_color else {
            return String::new();
        };
        let background = Self::css_hex(color);
        let text = Self::TEXT_COLORS[usize::from(color.is_light())];
        format!(
            "GtkWidget *header = gtk_header_bar_new();\n  \
             gtk_widget_add_css_class(header, \"tachyon-theme\");\n  \
             gtk_window_set_titlebar(window, header);\n  \
             GtkCssProvider *provider = gtk_css_provider_new();\n  \
             gtk_css_provider_load_from_string(provider, \"headerbar.tachyon-theme, headerbar.tachyon-theme:backdrop {{ background: {background}; color: {text}; }}\");\n  \
             gtk_style_context_add_provider_for_display(gtk_widget_get_display(GTK_WIDGET(window)), GTK_STYLE_PROVIDER(provider), GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);\n  \
             g_object_unref(provider);"
        )
    }

    fn css_hex(color: NativeThemeColor) -> String {
        format!("#{:02x}{:02x}{:02x}", color.red, color.green, color.blue)
    }

    fn display(window: &WindowConfiguration) -> &'static str {
        if window.display.starts_fullscreen() {
            return "gtk_window_fullscreen(window);";
        }
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::LinuxWindowAppearance;
    use crate::native::config::WindowConfiguration;
    use crate::native::native_display::NativeDisplay;
    use crate::native::native_theme_color::NativeThemeColor;

    #[test]
    fn an_undeclared_window_keeps_the_default_size_and_nothing_else() {
        let source = LinuxWindowAppearance::source(&WindowConfiguration::default());
        assert!(source.contains("gtk_window_set_default_size(window, 1024, 768);"));
        assert!(!source.contains("gtk_widget_set_size_request"));
        assert!(!source.contains("gtk_header_bar_new"));
        assert!(!source.contains("gtk_window_fullscreen"));
        assert!(!source.contains("__WINDOW_"));
    }

    #[test]
    fn declared_size_minimum_theme_color_and_fullscreen_reach_the_gtk_window() {
        let window = WindowConfiguration {
            width: 900,
            height: 640,
            min_width: Some(720),
            theme_color: Some(NativeThemeColor {
                red: 0x1a,
                green: 0x23,
                blue: 0x7e,
            }),
            display: NativeDisplay::Fullscreen,
            ..WindowConfiguration::default()
        };
        let source = LinuxWindowAppearance::source(&window);
        assert!(source.contains("gtk_window_set_default_size(window, 900, 640);"));
        assert!(source.contains("gtk_widget_set_size_request(content, 720, -1);"));
        assert!(source.contains("gtk_window_set_titlebar(window, header);"));
        assert!(source.contains("{ background: #1a237e; color: #ffffff; }"));
        assert!(source.contains("gtk_window_fullscreen(window);"));
    }

    #[test]
    fn a_light_theme_color_takes_black_header_bar_text() {
        let window = WindowConfiguration {
            theme_color: Some(NativeThemeColor {
                red: 0xff,
                green: 0xd7,
                blue: 0x00,
            }),
            ..WindowConfiguration::default()
        };
        let source = LinuxWindowAppearance::source(&window);
        assert!(source.contains("{ background: #ffd700; color: #000000; }"));
    }
}
