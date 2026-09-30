use super::config::WindowConfiguration;

pub(super) struct LinuxWindowControls;

impl LinuxWindowControls {
    const SOURCE: &'static str = include_str!("linux_window_controls.c");

    pub(super) fn source(window: &WindowConfiguration) -> String {
        let checks: Vec<String> = WindowConfiguration::KNOWN
            .iter()
            .filter(|control| window.grants(control))
            .map(|control| format!("if (strcmp(control, \"{control}\") == 0) return TRUE;"))
            .collect();
        Self::SOURCE.replace("__PERMITTED_OPERATIONS__", &checks.join("\n  "))
    }
}

#[cfg(test)]
mod tests {
    use super::{LinuxWindowControls, WindowConfiguration};

    #[test]
    fn linux_native_window_controls_embed_only_manifest_grants() {
        let denied = LinuxWindowControls::source(&WindowConfiguration::default());
        assert!(!denied.contains("return TRUE;"));
        for control in WindowConfiguration::KNOWN {
            let window = WindowConfiguration {
                controls: vec![String::from(control)],
                ..WindowConfiguration::default()
            };
            let source = LinuxWindowControls::source(&window);
            assert!(source.contains(&format!(
                "if (strcmp(control, \"{control}\") == 0) return TRUE;"
            )));
            assert_eq!(source.matches("return TRUE;").count(), 1);
            assert!(!source.contains("__PERMITTED_OPERATIONS__"));
        }
    }
}
