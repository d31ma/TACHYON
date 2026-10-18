use super::config::NativeApplication;
use tachyon_contracts::NativeTarget;

pub(super) struct AppleIcons;

impl AppleIcons {
    const APPLE_TARGETS: [NativeTarget; 2] = [NativeTarget::Macos, NativeTarget::Ios];

    pub(super) fn can_convert() -> bool {
        cfg!(target_os = "macos")
    }

    pub(super) fn skipped_note(
        application: &NativeApplication,
        target: NativeTarget,
    ) -> Option<String> {
        let skipped = [
            Self::APPLE_TARGETS.contains(&target),
            !Self::can_convert(),
            application.largest_icon().is_some(),
        ] == [true; 3];
        skipped.then(|| Self::note(target))
    }

    fn note(target: NativeTarget) -> String {
        let platform = super::compiler::native_target_directory(target);
        format!(
            "Note: the {platform} app was staged without its icon. Icon conversion uses sips and iconutil, which only macOS has. Build on a Mac to include the icon."
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::AppleIcons;
    use crate::native::config::NativeApplication;
    use std::fs;
    use tachyon_contracts::NativeTarget;

    fn application(manifest: &str) -> NativeApplication {
        let root = tempfile::tempdir().expect("project");
        fs::create_dir_all(root.path().join("client")).expect("client");
        fs::write(root.path().join("client/manifest.json"), manifest).expect("manifest");
        NativeApplication::discover(root.path()).expect("application")
    }

    #[test]
    fn an_apple_icon_staged_off_a_mac_gets_a_note_and_nothing_else_does() {
        let with_icon =
            application(r#"{"icons":[{"src":"/shared/icons/app.png","sizes":"512x512"}]}"#);
        let without_icon = application(r#"{"icons":[]}"#);
        for target in [NativeTarget::Macos, NativeTarget::Ios] {
            let note = AppleIcons::skipped_note(&with_icon, target);
            assert_eq!(note.is_some(), !AppleIcons::can_convert(), "{target:?}");
            if let Some(note) = note {
                let platform = crate::native::native_target_directory(target);
                assert!(note.contains(&format!("the {platform} app")), "{note}");
            }
            assert_eq!(AppleIcons::skipped_note(&without_icon, target), None);
        }
        for target in [
            NativeTarget::Windows,
            NativeTarget::Linux,
            NativeTarget::Android,
        ] {
            assert_eq!(AppleIcons::skipped_note(&with_icon, target), None);
        }
    }
}
