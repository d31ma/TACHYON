use super::browser_api_permissions::BrowserApiPermissions;

pub(super) struct LinuxMediaPackages;

impl LinuxMediaPackages {
    const MEDIA_GRANTS: [&'static str; 3] = ["camera", "microphone", "screen"];
    const RECOMMENDS: &'static str = "Recommends: xdg-desktop-portal, pipewire, gstreamer1.0-pipewire, gstreamer1.0-plugins-good\n";

    pub(super) fn recommends(permissions: &BrowserApiPermissions) -> &'static str {
        ["", Self::RECOMMENDS][usize::from(Self::media_granted(permissions))]
    }

    fn media_granted(permissions: &BrowserApiPermissions) -> bool {
        Self::MEDIA_GRANTS
            .iter()
            .any(|grant| permissions.is_granted(grant))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::LinuxMediaPackages;
    use crate::native::browser_api_permissions::BrowserApiPermissions;

    fn permissions(granted: &[&str]) -> BrowserApiPermissions {
        let granted: Vec<String> = granted.iter().map(|api| String::from(*api)).collect();
        BrowserApiPermissions::declared(&granted, &std::collections::BTreeMap::new())
    }

    #[test]
    fn a_camera_grant_recommends_the_portal_pipewire_and_gstreamer() {
        assert_eq!(
            LinuxMediaPackages::recommends(&permissions(&["camera"])),
            "Recommends: xdg-desktop-portal, pipewire, gstreamer1.0-pipewire, gstreamer1.0-plugins-good\n"
        );
    }

    #[test]
    fn a_microphone_grant_recommends_the_media_packages() {
        assert!(
            LinuxMediaPackages::recommends(&permissions(&["microphone"]))
                .starts_with("Recommends: xdg-desktop-portal,")
        );
    }

    #[test]
    fn a_screen_grant_recommends_the_media_packages() {
        assert!(
            LinuxMediaPackages::recommends(&permissions(&["screen", "share"]))
                .starts_with("Recommends: xdg-desktop-portal,")
        );
    }

    #[test]
    fn an_app_without_a_media_grant_recommends_nothing() {
        assert_eq!(
            LinuxMediaPackages::recommends(&permissions(&["share", "crypto"])),
            ""
        );
    }
}
