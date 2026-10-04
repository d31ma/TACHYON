use clap::ValueEnum;
use tachyon_core::WindowsArchitecture;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ArchitectureChoice {
    #[value(aliases = ["amd64", "x86_64", "x86-64"])]
    X64,
    #[value(alias = "aarch64")]
    Arm64,
    Native,
}

impl ArchitectureChoice {
    pub const fn windows(choice: Option<Self>) -> WindowsArchitecture {
        match choice {
            Some(Self::X64) => WindowsArchitecture::X64,
            Some(Self::Arm64) => WindowsArchitecture::Arm64,
            Some(Self::Native) | None => WindowsArchitecture::build_machine(),
        }
    }
}
