use crate::BuildTarget;
use tachyon_contracts::NativeTarget;
use tachyon_core::{Failure, ReleasePackaging};

#[derive(Clone, Copy, Debug, Default)]
pub struct ReleaseChoice;

impl ReleaseChoice {
    const EVERY_NATIVE_TARGET: [NativeTarget; 5] = [
        NativeTarget::Macos,
        NativeTarget::Windows,
        NativeTarget::Linux,
        NativeTarget::Android,
        NativeTarget::Ios,
    ];

    pub fn for_targets(
        release: bool,
        stages_only: bool,
        targets: &[BuildTarget],
    ) -> Result<bool, Failure> {
        let natives = Self::native_targets(targets);
        Self::refuse_staging(release, stages_only)?;
        Self::refuse_web_only(release, &natives)?;
        natives
            .iter()
            .try_for_each(|target| ReleasePackaging::check(*target, release))?;
        Ok(release)
    }

    fn refuse_staging(release: bool, stages_only: bool) -> Result<(), Failure> {
        let refused = [release, stages_only] == [true, true];
        (!refused).then_some(()).ok_or_else(|| {
            crate::cli_failure(
                1001,
                "--release builds a signed package, but --native only stages the host project.",
                "Remove --native to build the release package, or remove --release to stage the project.",
            )
        })
    }

    fn refuse_web_only(release: bool, natives: &[NativeTarget]) -> Result<(), Failure> {
        let refused = [release, natives.is_empty()] == [true, true];
        (!refused).then_some(()).ok_or_else(|| {
            crate::cli_failure(
                1001,
                "--release builds a signed native package, but no native target is being built. The web bundle is always a release build.",
                "Add --target android, or remove --release.",
            )
        })
    }

    fn native_targets(targets: &[BuildTarget]) -> Vec<NativeTarget> {
        targets
            .iter()
            .flat_map(|target| Self::natives_of(*target))
            .collect()
    }

    fn natives_of(target: BuildTarget) -> Vec<NativeTarget> {
        match target {
            BuildTarget::All => Self::EVERY_NATIVE_TARGET.to_vec(),
            other => other.native().into_iter().collect(),
        }
    }
}
