use crate::BuildTarget;
use tachyon_core::Failure;

#[derive(Clone, Copy, Debug, Default)]
pub struct DeviceChoice;

impl DeviceChoice {
    const IOS_TARGETS: [BuildTarget; 2] = [BuildTarget::Ios, BuildTarget::All];
    const UNCHANGED: [BuildTarget; 5] = [
        BuildTarget::Macos,
        BuildTarget::Linux,
        BuildTarget::Windows,
        BuildTarget::Android,
        BuildTarget::All,
    ];
    const NOTE: &'static str =
        "Note: --device applies only to the iOS app; the other native apps build as usual.";

    pub fn for_targets(
        device: bool,
        stages_only: bool,
        targets: &[BuildTarget],
    ) -> Result<bool, Failure> {
        Self::refuse_staging(device, stages_only)?;
        Self::refuse_without_ios(device, targets)?;
        Ok(device)
    }

    pub fn print_note(device: bool, targets: &[BuildTarget]) {
        Self::print(Self::note(device, targets));
    }

    fn print(note: Option<&str>) {
        if let Some(note) = note {
            println!("{note}");
        }
    }

    fn note(device: bool, targets: &[BuildTarget]) -> Option<&'static str> {
        let others = targets
            .iter()
            .any(|target| Self::UNCHANGED.contains(target));
        ([device, others] == [true, true]).then_some(Self::NOTE)
    }

    fn refuse_staging(device: bool, stages_only: bool) -> Result<(), Failure> {
        let refused = [device, stages_only] == [true, true];
        (!refused).then_some(()).ok_or_else(|| {
            crate::cli_failure(
                1001,
                "--device builds and signs the iOS app for a physical device, but --native only stages the host project.",
                "Remove --native to build for a device, or remove --device to stage the project.",
            )
        })
    }

    fn refuse_without_ios(device: bool, targets: &[BuildTarget]) -> Result<(), Failure> {
        let ios = targets
            .iter()
            .any(|target| Self::IOS_TARGETS.contains(target));
        let refused = [device, ios] == [true, false];
        (!refused).then_some(()).ok_or_else(|| {
            crate::cli_failure(
                1001,
                "--device builds the iOS app for a physical iPhone or iPad, but no iOS app is being built.",
                "Add --target ios, or remove --device.",
            )
        })
    }
}
