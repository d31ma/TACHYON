use super::config::WindowConfiguration;

pub(super) struct WindowsTitleBar;

impl WindowsTitleBar {
    const SOURCE: &'static str = include_str!("windows_title_bar.c");
    const PAGE_SCRIPT: &'static str = include_str!("title_bar/tac-title-bar.js");
    const PAGE_START: &'static str = "\nglobalThis.__tachyonTitleBar.useAppRegions();\nglobalThis.__tachyonHostPost('window.titleBarInsets', '{}').then((answer) => {\n  const insets = JSON.parse(answer);\n  if (typeof insets.height === 'number') globalThis.__tachyonTitleBar.apply(insets);\n}).catch(() => {});\n";
    const INTEGRATED: [&'static str; 2] = ["FALSE", "TRUE"];
    const WINDOW_TOP: [&'static str; 2] = ["bounds.top", "0"];
    const RESIZE_HEIGHT: &'static str = "bounds.bottom - bounds.top";
    const TITLE: &'static str = "SetWindowTextW(g_window, (const wchar_t *)title);";
    const ACTIONS: [fn(&str) -> String; 2] = [str::to_owned, Self::integrated_action];
    const INTEGRATED_TITLE: &'static str = "if (title[0] == 0) { wchar_t *name = tachyon_widen(TACHYON_APP_NAME); SetWindowTextW(g_window, name != NULL ? name : L\"\"); free(name); } else SetWindowTextW(g_window, (const wchar_t *)title);";

    pub(super) fn source(window: &WindowConfiguration) -> String {
        let integrated = window.title_bar.is_integrated();
        Self::SOURCE
            .replace(
                "__TITLE_BAR_INTEGRATED__",
                Self::INTEGRATED[usize::from(integrated)],
            )
            .replace("__TITLE_BAR_PAGE_SCRIPT__", &Self::page_script(integrated))
    }

    pub(super) fn window_top(window: &WindowConfiguration) -> &'static str {
        Self::WINDOW_TOP[usize::from(window.title_bar.is_integrated())]
    }

    pub(super) fn action(window: &WindowConfiguration, body: &str) -> String {
        Self::ACTIONS[usize::from(window.title_bar.is_integrated())](body)
    }

    fn integrated_action(body: &str) -> String {
        body.replace(Self::RESIZE_HEIGHT, "bounds.bottom")
            .replace(Self::TITLE, Self::INTEGRATED_TITLE)
    }

    fn page_script(integrated: bool) -> String {
        if integrated {
            let script = format!("{}{}", Self::PAGE_SCRIPT, Self::PAGE_START);
            return super::host::c_string_escape(&script).replace('?', "\\?");
        }
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::WindowsTitleBar;
    use crate::native::config::WindowConfiguration;
    use crate::native::native_title_bar::NativeTitleBar;

    fn integrated() -> WindowConfiguration {
        WindowConfiguration {
            title_bar: NativeTitleBar::Integrated,
            ..WindowConfiguration::default()
        }
    }

    #[test]
    fn a_standard_window_keeps_the_system_title_bar_and_answers_no_insets() {
        let window = WindowConfiguration::default();
        let source = WindowsTitleBar::source(&window);
        assert!(source.contains("TACHYON_TITLE_BAR_INTEGRATED = FALSE;"));
        assert!(source.contains("TACHYON_TITLE_BAR_PAGE_SCRIPT = \"\";"));
        assert!(source.contains("if (!TACHYON_TITLE_BAR_INTEGRATED) return TRUE;"));
        assert!(source.contains("Native title bar unavailable."));
        assert!(!source.contains("__TITLE_BAR_"));
        assert_eq!(WindowsTitleBar::window_top(&window), "bounds.top");
        let title = "SetWindowTextW(g_window, (const wchar_t *)title);";
        assert_eq!(WindowsTitleBar::action(&window, title), title);
    }

    #[test]
    fn an_integrated_window_extends_the_frame_and_gives_drag_regions_to_web_view2() {
        let source = WindowsTitleBar::source(&integrated());
        assert!(source.contains("TACHYON_TITLE_BAR_INTEGRATED = TRUE;"));
        assert!(source.contains("DwmExtendFrameIntoClientArea(window, &margins);"));
        assert!(source.contains("DwmDefWindowProc(window, WM_NCHITTEST, wparam, lparam, &hit)"));
        assert!(source.contains("DWMWA_CAPTION_BUTTON_BOUNDS"));
        assert!(source.contains("CombineRgn(region, region, hole, RGN_DIFF);"));
        assert!(source.contains(
            "ICoreWebView2Settings9_put_IsNonClientRegionSupportEnabled(regions, TRUE);"
        ));
        assert!(source.contains("GetSystemMenu(window, FALSE)"));
        assert!(source.contains("SetMenu(window, NULL);"));
        assert!(
            source.contains("globalThis.__tachyonTitleBar \\?\\?= new (class TacTitleBar {\\n")
        );
        assert!(source.contains("globalThis.__tachyonTitleBar.useAppRegions();"));
        assert!(source.contains("'window.titleBarInsets'"));
        assert!(source.contains("app-region: drag;"));
        assert!(!source.contains("__TITLE_BAR_"));
    }

    #[test]
    fn an_integrated_window_sizes_its_client_area_without_the_caption_and_keeps_its_name() {
        let window = integrated();
        assert_eq!(WindowsTitleBar::window_top(&window), "0");
        let resize = WindowsTitleBar::action(
            &window,
            "SetWindowPos(g_window, NULL, 0, 0, bounds.right - bounds.left, bounds.bottom - bounds.top, SWP_NOMOVE | SWP_NOZORDER);",
        );
        assert!(resize.contains("bounds.right - bounds.left, bounds.bottom, SWP_NOMOVE"));
        let title =
            WindowsTitleBar::action(&window, "SetWindowTextW(g_window, (const wchar_t *)title);");
        assert!(
            title.starts_with(
                "if (title[0] == 0) { wchar_t *name = tachyon_widen(TACHYON_APP_NAME);"
            )
        );
    }
}
