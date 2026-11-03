use super::config::WindowConfiguration;
use super::native_title_bar_button_inset::NativeTitleBarButtonInset;

pub(super) struct LinuxTitleBar;

impl LinuxTitleBar {
    const SOURCE: &'static str = include_str!("linux_title_bar.c");
    const PAGE_SCRIPT: &'static str = include_str!("title_bar/tac-title-bar.js");
    const INTEGRATED: [&'static str; 2] = ["FALSE", "TRUE"];
    const DEFAULT_INSET: NativeTitleBarButtonInset =
        NativeTitleBarButtonInset { leading: 6, top: 6 };

    pub(super) fn source(window: &WindowConfiguration) -> String {
        let integrated = window.title_bar.is_integrated();
        let inset = window.title_bar_button_inset.unwrap_or(Self::DEFAULT_INSET);
        Self::SOURCE
            .replace(
                "__TITLE_BAR_INTEGRATED__",
                Self::INTEGRATED[usize::from(integrated)],
            )
            .replace("__TITLE_BAR_TOP__", &inset.top.to_string())
            .replace("__TITLE_BAR_SIDE__", &inset.leading.to_string())
            .replace("__TITLE_BAR_PAGE_SCRIPT__", &Self::page_script(integrated))
    }

    fn page_script(integrated: bool) -> String {
        if integrated {
            return super::host::c_string_escape(Self::PAGE_SCRIPT).replace('?', "\\?");
        }
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::LinuxTitleBar;
    use crate::native::config::WindowConfiguration;
    use crate::native::native_title_bar::NativeTitleBar;
    use crate::native::native_title_bar_button_inset::NativeTitleBarButtonInset;

    #[test]
    fn a_standard_window_keeps_the_window_manager_title_bar_and_refuses_drag_regions() {
        let source = LinuxTitleBar::source(&WindowConfiguration::default());
        assert!(source.contains("TACHYON_TITLE_BAR_INTEGRATED = FALSE;"));
        assert!(source.contains("TACHYON_TITLE_BAR_PAGE_SCRIPT = \"\";"));
        assert!(source.contains("if (!TACHYON_TITLE_BAR_INTEGRATED) return GTK_WIDGET(view);"));
        assert!(source.contains("Native title bar unavailable."));
        assert!(!source.contains("__TITLE_BAR_"));
    }

    #[test]
    fn an_integrated_window_overlays_gtk_window_buttons_and_injects_the_page_script() {
        let window = WindowConfiguration {
            title_bar: NativeTitleBar::Integrated,
            ..WindowConfiguration::default()
        };
        let source = LinuxTitleBar::source(&window);
        assert!(source.contains("TACHYON_TITLE_BAR_INTEGRATED = TRUE;"));
        assert!(source.contains("TACHYON_TITLE_BAR_TOP = 6;"));
        assert!(source.contains("TACHYON_TITLE_BAR_SIDE = 6;"));
        assert!(
            source.contains("globalThis.__tachyonTitleBar \\?\\?= new (class TacTitleBar {\\n")
        );
        assert!(source.contains("'window.dragRegions'"));
        assert!(source.contains("gtk_window_controls_new(GTK_PACK_START)"));
        assert!(source.contains("gtk_window_controls_new(GTK_PACK_END)"));
        assert!(source.contains("gtk_window_set_titlebar(window, nothing);"));
        assert!(source.contains(
            "gtk_application_window_set_show_menubar(GTK_APPLICATION_WINDOW(window), FALSE);"
        ));
        assert!(source.contains("gdk_toplevel_begin_move"));
        assert!(source.contains("\"gtk-titlebar-double-click\""));
        assert!(!source.contains("__TITLE_BAR_"));
    }

    #[test]
    fn a_button_inset_places_the_linux_window_buttons() {
        let window = WindowConfiguration {
            title_bar: NativeTitleBar::Integrated,
            title_bar_button_inset: Some(NativeTitleBarButtonInset {
                leading: 20,
                top: 35,
            }),
            ..WindowConfiguration::default()
        };
        let source = LinuxTitleBar::source(&window);
        assert!(source.contains("TACHYON_TITLE_BAR_TOP = 35;"));
        assert!(source.contains("TACHYON_TITLE_BAR_SIDE = 20;"));
    }
}
