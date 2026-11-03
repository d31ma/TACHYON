use super::config::{NativeApplication, config_failure};
use crate::Failure;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Copy, Debug)]
pub(super) struct NativeReleaseVersion<'application> {
    version: &'application str,
    build: u32,
}

impl<'application> NativeReleaseVersion<'application> {
    const MAX_NUMBERS: usize = 3;
    const MAX_WINDOWS_NUMBER: u32 = 65_535;
    const MAX_ANDROID_BUILD: u32 = 2_100_000_000;

    pub(super) fn of(application: &'application NativeApplication) -> Self {
        Self {
            version: &application.version,
            build: application.build,
        }
    }

    pub(super) fn validate_for(self, target: NativeTarget) -> Result<(), Failure> {
        self.require_apple_version(target)?;
        self.require_windows_version(target)?;
        self.require_windows_build(target)?;
        self.require_android_build(target)
    }

    pub(super) fn version(self) -> &'application str {
        self.version
    }

    pub(super) fn windows_numbers(self) -> [u32; 4] {
        let declared = self.numbers().unwrap_or_default();
        let number = |index: usize| declared.get(index).copied().unwrap_or(0);
        [number(0), number(1), number(2), self.build]
    }

    pub(super) fn windows_dotted(self) -> String {
        self.windows_numbers()
            .map(|number| number.to_string())
            .join(".")
    }

    fn numbers(self) -> Option<Vec<u32>> {
        let numbers = self
            .version
            .split('.')
            .map(Self::number)
            .collect::<Option<Vec<u32>>>()?;
        (numbers.len() <= Self::MAX_NUMBERS).then_some(numbers)
    }

    fn number(segment: &str) -> Option<u32> {
        segment
            .bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| segment.parse::<u32>().ok())
            .flatten()
    }

    fn windows_version_fits(self) -> bool {
        self.numbers().is_some_and(|numbers| {
            numbers
                .iter()
                .all(|number| *number <= Self::MAX_WINDOWS_NUMBER)
        })
    }

    fn require_apple_version(self, target: NativeTarget) -> Result<(), Failure> {
        let platform = super::compiler::native_target_directory(target);
        let applies = [NativeTarget::Macos, NativeTarget::Ios].contains(&target);
        (!applies || self.numbers().is_some())
            .then_some(())
            .ok_or_else(|| {
                config_failure(&format!(
                    "Manifest tac.application.version '{}' cannot be the {platform} app's version. \
                     Apple apps need one to three numbers separated by dots, such as 1.4.0. \
                     Leave a label such as -beta out of it.",
                    self.version
                ))
            })
    }

    fn require_windows_version(self, target: NativeTarget) -> Result<(), Failure> {
        (target != NativeTarget::Windows || self.windows_version_fits())
            .then_some(())
            .ok_or_else(|| {
                config_failure(&format!(
                    "Manifest tac.application.version '{}' cannot be the windows app's file version. \
                     Windows needs one to three numbers from 0 to {} separated by dots, such as 1.4.0; \
                     tac.application.build is the fourth number.",
                    self.version,
                    Self::MAX_WINDOWS_NUMBER
                ))
            })
    }

    fn require_windows_build(self, target: NativeTarget) -> Result<(), Failure> {
        (target != NativeTarget::Windows || self.build <= Self::MAX_WINDOWS_NUMBER)
            .then_some(())
            .ok_or_else(|| {
                config_failure(&format!(
                    "Manifest tac.application.build {} is too large for the windows app. \
                     It is the fourth number of the Windows file version, which is at most {}.",
                    self.build,
                    Self::MAX_WINDOWS_NUMBER
                ))
            })
    }

    fn require_android_build(self, target: NativeTarget) -> Result<(), Failure> {
        (target != NativeTarget::Android || self.build <= Self::MAX_ANDROID_BUILD)
            .then_some(())
            .ok_or_else(|| {
                config_failure(&format!(
                    "Manifest tac.application.build {} is too large for the android app. \
                     It is the versionCode, which Google Play accepts up to {}.",
                    self.build,
                    Self::MAX_ANDROID_BUILD
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::NativeReleaseVersion;
    use tachyon_contracts::NativeTarget;

    fn release(version: &str, build: u32) -> NativeReleaseVersion<'_> {
        NativeReleaseVersion { version, build }
    }

    fn refusal(version: &str, build: u32, target: NativeTarget) -> String {
        release(version, build)
            .validate_for(target)
            .expect_err("refused")
            .to_string()
    }

    #[test]
    fn numeric_versions_build_on_every_target() {
        for version in ["1", "1.4", "1.4.0", "0.0.1", "65535.65535.65535"] {
            for target in [
                NativeTarget::Macos,
                NativeTarget::Ios,
                NativeTarget::Android,
                NativeTarget::Windows,
                NativeTarget::Linux,
            ] {
                assert!(
                    release(version, 7).validate_for(target).is_ok(),
                    "{version} on {target:?}"
                );
            }
        }
    }

    #[test]
    fn a_labelled_version_builds_only_where_the_platform_allows_text() {
        for version in ["1.4.0-beta", "1.4.0+5", "2026.10a", "1.2.3.4", "1..2", "v1"] {
            for target in [NativeTarget::Android, NativeTarget::Linux] {
                assert!(release(version, 1).validate_for(target).is_ok());
            }
            for target in [NativeTarget::Macos, NativeTarget::Ios] {
                let text = refusal(version, 1, target);
                assert!(text.contains("TY1601"), "{text}");
                assert!(
                    text.contains("Apple apps need one to three numbers"),
                    "{text}"
                );
            }
            let text = refusal(version, 1, NativeTarget::Windows);
            assert!(text.contains("TY1601"), "{text}");
            assert!(
                text.contains("tac.application.build is the fourth number"),
                "{text}"
            );
        }
    }

    #[test]
    fn windows_refuses_numbers_beyond_its_file_version() {
        let text = refusal("1.65536.0", 1, NativeTarget::Windows);
        assert!(text.contains("from 0 to 65535"), "{text}");
        let text = refusal("1.4.0", 65_536, NativeTarget::Windows);
        assert!(
            text.contains("build 65536 is too large for the windows app"),
            "{text}"
        );
        assert!(
            release("1.65536.0", 65_536)
                .validate_for(NativeTarget::Macos)
                .is_ok()
        );
    }

    #[test]
    fn android_refuses_a_build_beyond_the_largest_version_code() {
        let text = refusal("1.4.0", 2_100_000_001, NativeTarget::Android);
        assert!(text.contains("versionCode"), "{text}");
        assert!(
            release("1.4.0", 2_100_000_000)
                .validate_for(NativeTarget::Android)
                .is_ok()
        );
    }

    #[test]
    fn the_windows_file_version_is_the_version_padded_then_the_build() {
        assert_eq!(release("1.4", 12).windows_numbers(), [1, 4, 0, 12]);
        assert_eq!(release("3", 1).windows_dotted(), "3.0.0.1");
        assert_eq!(release("2.5.9", 300).windows_dotted(), "2.5.9.300");
    }
}
