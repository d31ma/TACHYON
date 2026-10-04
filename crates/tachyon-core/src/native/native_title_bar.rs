use serde::Deserialize;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(super) enum NativeTitleBar {
    #[default]
    Standard,
    Integrated,
}

impl NativeTitleBar {
    pub(super) fn is_integrated(self) -> bool {
        self == Self::Integrated
    }

    pub(super) fn fallback_note(self, target: NativeTarget) -> Option<String> {
        self.falls_back_on(target).then(|| Self::note(target))
    }

    fn falls_back_on(self, target: NativeTarget) -> bool {
        self.is_integrated() && target != NativeTarget::Macos
    }

    fn note(target: NativeTarget) -> String {
        let platform = super::compiler::native_target_directory(target);
        format!(
            "Note: tac.application.window.titleBar \"integrated\" is built only for macOS. The {platform} app keeps the standard title bar (ADR 0099)."
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::NativeTitleBar;
    use tachyon_contracts::NativeTarget;

    #[test]
    fn only_an_integrated_title_bar_outside_macos_gets_a_fallback_note() {
        assert_eq!(
            NativeTitleBar::Integrated.fallback_note(NativeTarget::Macos),
            None
        );
        assert_eq!(
            NativeTitleBar::Standard.fallback_note(NativeTarget::Windows),
            None
        );
        for target in [
            NativeTarget::Windows,
            NativeTarget::Linux,
            NativeTarget::Ios,
            NativeTarget::Android,
        ] {
            let note = NativeTitleBar::Integrated
                .fallback_note(target)
                .expect("fallback note");
            let platform = super::super::compiler::native_target_directory(target);
            assert!(note.contains(&format!("The {platform} app keeps the standard title bar")));
        }
    }
}
