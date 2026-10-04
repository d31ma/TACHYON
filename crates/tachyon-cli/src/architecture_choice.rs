use crate::BuildTarget;
use clap::ValueEnum;
use tachyon_core::{Failure, WindowsArchitecture};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ArchitectureChoice {
    #[value(aliases = ["amd64", "x86_64", "x86-64"])]
    X64,
    #[value(alias = "aarch64")]
    Arm64,
    Native,
}

impl ArchitectureChoice {
    const WINDOWS_TARGETS: [BuildTarget; 2] = [BuildTarget::Windows, BuildTarget::All];

    pub fn for_targets(
        choice: Option<Self>,
        targets: &[BuildTarget],
    ) -> Result<WindowsArchitecture, Failure> {
        let builds_windows = targets
            .iter()
            .any(|target| Self::WINDOWS_TARGETS.contains(target));
        let refused = [choice.is_some(), !builds_windows] == [true, true];
        (!refused).then(|| Self::windows(choice)).ok_or_else(|| {
            crate::cli_failure(
                1001,
                "--arch chooses the Windows host architecture, but no Windows target is being built.",
                "Add --target windows, or remove --arch.",
            )
        })
    }

    pub fn windows(choice: Option<Self>) -> WindowsArchitecture {
        choice.map_or_else(WindowsArchitecture::build_machine, Self::chosen)
    }

    fn chosen(self) -> WindowsArchitecture {
        match self {
            Self::X64 => WindowsArchitecture::X64,
            Self::Arm64 => WindowsArchitecture::ARM64,
            Self::Native => WindowsArchitecture::build_machine(),
        }
    }
}
