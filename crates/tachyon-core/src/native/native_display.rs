use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum NativeDisplay {
    Fullscreen,
    #[default]
    Standalone,
    MinimalUi,
    Browser,
}

impl NativeDisplay {
    pub(super) fn starts_fullscreen(self) -> bool {
        self == Self::Fullscreen
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::NativeDisplay;

    #[test]
    fn the_four_web_display_modes_parse_and_only_fullscreen_starts_fullscreen() {
        for (text, display, fullscreen) in [
            ("fullscreen", NativeDisplay::Fullscreen, true),
            ("standalone", NativeDisplay::Standalone, false),
            ("minimal-ui", NativeDisplay::MinimalUi, false),
            ("browser", NativeDisplay::Browser, false),
        ] {
            let parsed: NativeDisplay =
                serde_json::from_value(serde_json::json!(text)).expect("display mode");
            assert_eq!(parsed, display);
            assert_eq!(parsed.starts_fullscreen(), fullscreen);
        }
        assert!(serde_json::from_value::<NativeDisplay>(serde_json::json!("kiosk")).is_err());
        assert_eq!(NativeDisplay::default(), NativeDisplay::Standalone);
    }
}
