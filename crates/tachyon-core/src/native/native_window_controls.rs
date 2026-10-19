use super::config::WindowConfiguration;
use std::fmt::Write;

pub(super) struct NativeWindowControls;

impl NativeWindowControls {
    const WINDOWS_ACTIONS: [(&'static str, &'static str); 6] = [
        ("minimize", "ShowWindow(g_window, SW_MINIMIZE);"),
        (
            "maximize",
            "ShowWindow(g_window, IsZoomed(g_window) ? SW_RESTORE : SW_MAXIMIZE);",
        ),
        ("close", "PostMessageW(g_window, WM_CLOSE, 0, 0);"),
        (
            "resize",
            r#"int width = tachyon_window_size(payload, "width"), height = tachyon_window_size(payload, "height"); if (!width || !height) return _strdup("{\"ok\":false,\"error\":\"Invalid window size.\"}"); RECT bounds = {0, 0, width, height}; AdjustWindowRect(&bounds, (DWORD)GetWindowLongPtrW(g_window, GWL_STYLE), FALSE); SetWindowPos(g_window, NULL, 0, 0, bounds.right - bounds.left, bounds.bottom - bounds.top, SWP_NOMOVE | SWP_NOZORDER);"#,
        ),
        (
            "title",
            r#"uint16_t title[513]; if (!tachyon_window_title(payload, title, 513)) return _strdup("{\"ok\":false,\"error\":\"Invalid window title.\"}"); SetWindowTextW(g_window, (const wchar_t *)title);"#,
        ),
        (
            "fullscreen",
            r#"if (!tachyon_window_toggle_fullscreen(g_window)) return _strdup("{\"ok\":false}");"#,
        ),
    ];

    const WINDOWS_FULLSCREEN: [&'static str; 2] =
        ["", include_str!("native_window_fullscreen_windows.c")];

    pub(super) fn android(window: &WindowConfiguration) -> String {
        let grants = Self::android_grants(window);
        include_str!("native_window_controls_android.java").replace("__WINDOW_GRANTS__", &grants)
    }

    pub(super) fn windows(window: &WindowConfiguration) -> String {
        let grants = Self::windows_grants(window);
        let actions = Self::windows_actions(window);
        include_str!("native_window_controls_windows.c")
            .replace(
                "__WINDOW_PAYLOAD__",
                include_str!("native_window_payload.h"),
            )
            .replace("__WINDOW_FULLSCREEN__", Self::windows_fullscreen(window))
            .replace("__WINDOW_GRANTS__", &grants)
            .replace("__WINDOW_ACTIONS__", &actions)
    }

    fn windows_fullscreen(window: &WindowConfiguration) -> &'static str {
        let needed = [
            window.grants("fullscreen"),
            window.display.starts_fullscreen(),
        ]
        .contains(&true);
        Self::WINDOWS_FULLSCREEN[usize::from(needed)]
    }

    fn android_grants(window: &WindowConfiguration) -> String {
        let mut output = String::new();
        for control in &window.controls {
            let _ = writeln!(output, "      case {control:?}: return true;");
        }
        output
    }

    fn windows_grants(window: &WindowConfiguration) -> String {
        let grants = window
            .controls
            .iter()
            .map(|control| {
                format!("tachyon_payload_string_matches(payload, \"control\", {control:?})")
            })
            .collect::<Vec<_>>()
            .join(" || ");
        if grants.is_empty() {
            String::from("0")
        } else {
            grants
        }
    }

    fn windows_actions(window: &WindowConfiguration) -> String {
        let actions = Self::granted_actions(window);
        let mut output = String::new();
        for (control, body) in actions {
            let body = super::windows_title_bar::WindowsTitleBar::action(window, body);
            let _ = writeln!(
                output,
                "  if (strcmp(capability, \"window.{control}\") == 0) {{ {body} return _strdup(\"{{\\\"ok\\\":true}}\"); }}"
            );
        }
        output
    }

    fn granted_actions(window: &WindowConfiguration) -> Vec<(&'static str, &'static str)> {
        Self::WINDOWS_ACTIONS
            .into_iter()
            .filter(|(control, _)| window.grants(control))
            .collect()
    }
}
