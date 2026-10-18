use super::target_support::TargetSupport;
use crate::Failure;
use crate::failure::diagnostic;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Copy, Debug, Default)]
pub struct ReleasePackaging;

impl ReleasePackaging {
    const PACKAGED: [NativeTarget; 1] = [NativeTarget::Android];

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

    fn refusal(target: NativeTarget) -> Failure {
        let platform = TargetSupport::native_platform(target);
        Failure::one(diagnostic(
            1601,
            format!(
                "The {platform} target has no release packaging yet, so --release cannot build it."
            ),
            Some(format!(
                "Build {platform} without --release. Signed release builds are available for --target android."
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
    fn only_android_builds_a_release_package() {
        assert!(ReleasePackaging::check(NativeTarget::Android, true).is_ok());
        for target in [
            NativeTarget::Macos,
            NativeTarget::Windows,
            NativeTarget::Linux,
            NativeTarget::Ios,
        ] {
            assert!(ReleasePackaging::check(target, false).is_ok());
            let text = ReleasePackaging::check(target, true)
                .expect_err("no release packaging")
                .to_string();
            assert!(text.contains("TY1601"));
            assert!(text.contains("--target android"));
        }
        let ios = ReleasePackaging::check(NativeTarget::Ios, true).expect_err("ios");
        assert!(
            ios.to_string()
                .contains("The iOS target has no release packaging yet")
        );
    }
}
