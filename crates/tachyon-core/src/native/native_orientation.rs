use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum NativeOrientation {
    #[default]
    Any,
    Natural,
    Landscape,
    LandscapePrimary,
    LandscapeSecondary,
    Portrait,
    PortraitPrimary,
    PortraitSecondary,
}

impl NativeOrientation {
    const IOS_PORTRAIT: &'static str = "UIInterfaceOrientationPortrait";
    const IOS_UPSIDE_DOWN: &'static str = "UIInterfaceOrientationPortraitUpsideDown";
    const IOS_LANDSCAPE_LEFT: &'static str = "UIInterfaceOrientationLandscapeLeft";
    const IOS_LANDSCAPE_RIGHT: &'static str = "UIInterfaceOrientationLandscapeRight";

    pub(super) fn ios_orientations(self) -> &'static [&'static str] {
        match self {
            Self::Any => &[
                Self::IOS_PORTRAIT,
                Self::IOS_UPSIDE_DOWN,
                Self::IOS_LANDSCAPE_LEFT,
                Self::IOS_LANDSCAPE_RIGHT,
            ],
            Self::Natural | Self::PortraitPrimary => &[Self::IOS_PORTRAIT],
            Self::Portrait => &[Self::IOS_PORTRAIT, Self::IOS_UPSIDE_DOWN],
            Self::PortraitSecondary => &[Self::IOS_UPSIDE_DOWN],
            Self::Landscape => &[Self::IOS_LANDSCAPE_LEFT, Self::IOS_LANDSCAPE_RIGHT],
            Self::LandscapePrimary => &[Self::IOS_LANDSCAPE_RIGHT],
            Self::LandscapeSecondary => &[Self::IOS_LANDSCAPE_LEFT],
        }
    }

    pub(super) fn android_screen_orientation(self) -> Option<&'static str> {
        match self {
            Self::Any => None,
            Self::Natural => Some("nosensor"),
            Self::Landscape => Some("userLandscape"),
            Self::LandscapePrimary => Some("landscape"),
            Self::LandscapeSecondary => Some("reverseLandscape"),
            Self::Portrait => Some("userPortrait"),
            Self::PortraitPrimary => Some("portrait"),
            Self::PortraitSecondary => Some("reversePortrait"),
        }
    }

    pub(super) fn is_locked(self) -> bool {
        self != Self::Any
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::NativeOrientation;

    fn parsed(text: &str) -> NativeOrientation {
        serde_json::from_value(serde_json::json!(text)).expect("orientation")
    }

    #[test]
    fn the_eight_web_orientations_parse_and_anything_else_is_refused() {
        for text in [
            "any",
            "natural",
            "landscape",
            "landscape-primary",
            "landscape-secondary",
            "portrait",
            "portrait-primary",
            "portrait-secondary",
        ] {
            let orientation = parsed(text);
            assert!(!orientation.ios_orientations().is_empty(), "{text}");
            assert_eq!(orientation.is_locked(), text != "any", "{text}");
        }
        assert!(
            serde_json::from_value::<NativeOrientation>(serde_json::json!("sideways")).is_err()
        );
        assert_eq!(NativeOrientation::default(), NativeOrientation::Any);
    }

    #[test]
    fn landscape_and_portrait_map_to_both_matching_turns_of_the_device() {
        assert_eq!(
            parsed("landscape").ios_orientations(),
            [
                "UIInterfaceOrientationLandscapeLeft",
                "UIInterfaceOrientationLandscapeRight"
            ]
        );
        assert_eq!(
            parsed("portrait").ios_orientations(),
            [
                "UIInterfaceOrientationPortrait",
                "UIInterfaceOrientationPortraitUpsideDown"
            ]
        );
        assert_eq!(parsed("any").ios_orientations().len(), 4);
        assert_eq!(
            parsed("landscape").android_screen_orientation(),
            Some("userLandscape")
        );
        assert_eq!(
            parsed("portrait").android_screen_orientation(),
            Some("userPortrait")
        );
        assert_eq!(parsed("any").android_screen_orientation(), None);
    }

    #[test]
    fn primary_landscape_turns_the_top_of_the_device_to_the_left() {
        assert_eq!(
            parsed("landscape-primary").ios_orientations(),
            ["UIInterfaceOrientationLandscapeRight"]
        );
        assert_eq!(
            parsed("landscape-primary").android_screen_orientation(),
            Some("landscape")
        );
        assert_eq!(
            parsed("portrait-secondary").android_screen_orientation(),
            Some("reversePortrait")
        );
    }
}
