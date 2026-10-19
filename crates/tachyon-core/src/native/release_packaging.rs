use super::target_support::TargetSupport;
use crate::Failure;
use crate::failure::diagnostic;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Copy, Debug, Default)]
pub struct ReleasePackaging;

impl ReleasePackaging {
    const PACKAGED: [NativeTarget; 5] = [
        NativeTarget::Android,
        NativeTarget::Macos,
        NativeTarget::Linux,
        NativeTarget::Windows,
        NativeTarget::Ios,
    ];

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
        let packaged = Self::PACKAGED
            .map(TargetSupport::native_platform)
            .join(", ");
        Failure::one(diagnostic(
            1601,
            format!(
                "The {platform} target has no release packaging yet, so --release cannot build it."
            ),
            Some(format!(
                "Build {platform} without --release. Release packages are built for: {packaged}."
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
    fn every_native_target_builds_a_release_package() {
        for target in [
            NativeTarget::Android,
            NativeTarget::Macos,
            NativeTarget::Linux,
            NativeTarget::Windows,
            NativeTarget::Ios,
        ] {
            assert!(ReleasePackaging::check(target, true).is_ok());
        }
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
