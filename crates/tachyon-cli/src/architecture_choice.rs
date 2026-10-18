use crate::BuildTarget;
use crate::package_architectures::PackageArchitectures;
use clap::ValueEnum;
use tachyon_core::{AppTarget, Failure, MacOsArchitecture, TargetSupport, WindowsArchitecture};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ArchitectureChoice {
    #[value(aliases = ["amd64", "x86_64", "x86-64"])]
    X64,
    #[value(alias = "aarch64")]
    Arm64,
    Native,
    Universal,
}

impl ArchitectureChoice {
    const CAPABILITY: &'static str = "build architecture (--arch)";
    const BY_BUILD_MACHINE: [Self; 2] = [Self::X64, Self::Arm64];

    pub fn for_targets(
        choice: Option<Self>,
        targets: &[BuildTarget],
    ) -> Result<PackageArchitectures, Failure> {
        choice.map_or(Ok(PackageArchitectures::default()), |chosen| {
            chosen.checked(targets)
        })
    }

    pub fn notes(choice: Option<Self>, targets: &[BuildTarget]) -> Result<Vec<String>, Failure> {
        choice.map_or(Ok(Vec::new()), |chosen| chosen.notes_for(targets))
    }

    fn checked(self, targets: &[BuildTarget]) -> Result<PackageArchitectures, Failure> {
        let honored = targets.iter().any(|target| self.honors(*target));
        honored
            .then(|| self.architectures())
            .ok_or_else(|| self.refusal())
    }

    fn refusal(self) -> Failure {
        crate::cli_failure(
            1001,
            format!(
                "--arch {} cannot be honored by any target being built.",
                self.name()
            ),
            "Windows takes x64 or arm64, and macOS takes x64, arm64 or universal. Linux and iOS apps are built for the build machine's architecture, and web and Android builds take no --arch. Add --target windows or --target macos, or remove --arch.",
        )
    }

    fn notes_for(self, targets: &[BuildTarget]) -> Result<Vec<String>, Failure> {
        let notes: Vec<Option<String>> = targets
            .iter()
            .filter(|target| !self.honors(**target))
            .map(|target| Self::note(*target))
            .collect::<Result<_, _>>()?;
        Ok(notes.into_iter().flatten().collect())
    }

    fn honors(self, target: BuildTarget) -> bool {
        match target {
            BuildTarget::Macos => true,
            BuildTarget::Windows => self != Self::Universal,
            BuildTarget::Linux | BuildTarget::Ios => self.is_build_machine(),
            BuildTarget::Web | BuildTarget::Android | BuildTarget::All => false,
        }
    }

    fn is_build_machine(self) -> bool {
        let machine = Self::BY_BUILD_MACHINE[usize::from(cfg!(target_arch = "aarch64"))];
        [Self::Native, machine].contains(&self)
    }

    fn note(target: BuildTarget) -> Result<Option<String>, Failure> {
        if target == BuildTarget::Windows {
            return Ok(Some(Self::windows_note()));
        }
        target
            .app_target()
            .map(Self::table_note)
            .transpose()
            .map(Option::flatten)
    }

    fn windows_note() -> String {
        format!(
            "Note: --arch universal is for macOS only, so the Windows app is built for {}.",
            WindowsArchitecture::build_machine().name()
        )
    }

    fn table_note(target: AppTarget) -> Result<Option<String>, Failure> {
        let note = TargetSupport::capability_note(Self::CAPABILITY, target)?;
        Ok(note.map(|text| format!("Note: {text}")))
    }

    fn name(self) -> String {
        self.to_possible_value()
            .map(|value| value.get_name().to_owned())
            .unwrap_or_default()
    }

    fn architectures(self) -> PackageArchitectures {
        PackageArchitectures {
            windows: self.windows(),
            macos: self.macos(),
        }
    }

    fn windows(self) -> WindowsArchitecture {
        match self {
            Self::X64 => WindowsArchitecture::X64,
            Self::Arm64 => WindowsArchitecture::ARM64,
            Self::Native | Self::Universal => WindowsArchitecture::build_machine(),
        }
    }

    fn macos(self) -> MacOsArchitecture {
        match self {
            Self::X64 => MacOsArchitecture::X64,
            Self::Arm64 => MacOsArchitecture::ARM64,
            Self::Native => MacOsArchitecture::build_machine(),
            Self::Universal => MacOsArchitecture::UNIVERSAL,
        }
    }
}
