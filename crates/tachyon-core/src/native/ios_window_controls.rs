use super::config::WindowConfiguration;

pub(super) struct IosWindowControls;

impl IosWindowControls {
    const HONORED: [&'static str; 1] = ["title"];
    const GRANTED_CASE: &'static str = r#"        case "window.granted":
            let granted: [String] = __GRANTED__
            return granted.contains(parsed["control"] as? String ?? "") ? "{\"granted\":true}" : "{\"granted\":false}"
"#;
    const TITLE_CASE: &'static str = r#"        case "window.title":
            guard let title = parsed["title"] as? String, title.utf8.count <= 1024 else { return "{\"error\":\"invalid\"}" }
            webView?.window?.windowScene?.title = title
"#;

    pub(super) fn page_window(window: &WindowConfiguration) -> WindowConfiguration {
        WindowConfiguration {
            controls: Self::honored(window),
            ..window.clone()
        }
    }

    pub(super) fn bridge_cases(window: &WindowConfiguration) -> String {
        let granted =
            serde_json::to_string(&Self::honored(window)).unwrap_or_else(|_| String::from("[]"));
        let title = ["", Self::TITLE_CASE][usize::from(window.grants("title"))];
        format!(
            "{}{title}",
            Self::GRANTED_CASE.replace("__GRANTED__", &granted)
        )
    }

    fn honored(window: &WindowConfiguration) -> Vec<String> {
        window
            .controls
            .iter()
            .filter(|control| Self::HONORED.contains(&control.as_str()))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{IosWindowControls, WindowConfiguration};

    fn window(controls: &[&str]) -> WindowConfiguration {
        WindowConfiguration {
            controls: controls
                .iter()
                .map(|control| String::from(*control))
                .collect(),
            ..WindowConfiguration::default()
        }
    }

    #[test]
    fn ios_pages_see_only_the_title_grant_and_the_bridge_sets_the_scene_title() {
        let declared = window(&["minimize", "title", "close"]);
        assert_eq!(
            IosWindowControls::page_window(&declared).controls,
            vec![String::from("title")]
        );
        let cases = IosWindowControls::bridge_cases(&declared);
        assert!(cases.contains("let granted: [String] = [\"title\"]"));
        assert!(cases.contains("webView?.window?.windowScene?.title = title"));
        let untitled = IosWindowControls::bridge_cases(&window(&["minimize"]));
        assert!(untitled.contains("let granted: [String] = []"));
        assert!(!untitled.contains("case \"window.title\""));
    }
}
