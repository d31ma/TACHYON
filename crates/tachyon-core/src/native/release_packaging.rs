use super::target_support::TargetSupport;
use crate::Failure;
use crate::failure::diagnostic;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Copy, Debug, Default)]
pub struct ReleasePackaging;

impl ReleasePackaging {
    const PACKAGED: [NativeTarget; 2] = [NativeTarget::Android, NativeTarget::Macos];
    const PACKAGED_TARGETS: &'static str = "--target android and --target macos";

    /// Checks that the target can build a signed release package.
    ///
    /// # Errors
    ///
    /// Returns TY1601 when a release is asked for a target that has no release packaging.
    pub fn check(target: NativeTarget, release: bool) -> Result<(), Failure> {
        let refused = [release, !Self::PACKAGED.contains(&target)] == [true, true];
        (!refused)
            .then_some(())
            .ok_or_else(|| Self::refusal(target))
    }

    /// Checks the signing settings of a release before anything is built, and
    /// returns the note a release with test signing prints.
    ///
    /// # Errors
    ///
    /// Returns TY1605 when the environment lacks what the target's signing needs.
    pub fn signing_note(target: NativeTarget, release: bool) -> Result<Option<String>, Failure> {
        let signs_on_a_mac = [release, target == NativeTarget::Macos] == [true, true];
        let signing = super::mac_os_signing::MacOsSigning::for_release(signs_on_a_mac)?;
        Ok(signing.and_then(|signing| signing.note()))
    }

    fn refusal(target: NativeTarget) -> Failure {
        let platform = TargetSupport::native_platform(target);
        Failure::one(diagnostic(
            1601,
            format!(
                "The {platform} target has no release packaging yet, so --release cannot build it."
            ),
            Some(format!(
                "Build {platform} without --release. Signed release builds are available for {}.",
                Self::PACKAGED_TARGETS
            )),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::ReleasePackaging;
    use tachyon_contracts::NativeTarget;

    #[test]
    fn android_and_macos_build_a_release_package() {
        assert!(ReleasePackaging::check(NativeTarget::Android, true).is_ok());
        assert!(ReleasePackaging::check(NativeTarget::Macos, true).is_ok());
        for target in [
            NativeTarget::Windows,
            NativeTarget::Linux,
            NativeTarget::Ios,
        ] {
            assert!(ReleasePackaging::check(target, false).is_ok());
            let text = ReleasePackaging::check(target, true)
                .expect_err("no release packaging")
                .to_string();
            assert!(text.contains("TY1601"));
            assert!(text.contains("--target android and --target macos"));
        }
        let ios = ReleasePackaging::check(NativeTarget::Ios, true).expect_err("ios");
        assert!(
            ios.to_string()
                .contains("The iOS target has no release packaging yet")
        );
    }

    #[test]
    fn only_a_macos_release_reads_the_apple_signing_settings() {
        for (target, release) in [
            (NativeTarget::Android, true),
            (NativeTarget::Macos, false),
            (NativeTarget::Linux, true),
        ] {
            assert_eq!(
                ReleasePackaging::signing_note(target, release).expect("no signing check"),
                None
            );
        }
    }
}
