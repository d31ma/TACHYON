use tachyon_contracts::NativeTarget::{self, Android, Ios, Linux, Macos, Windows};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeCompanion {
    Rust,
    Swift,
    Kotlin,
    CSharp,
}

impl NativeCompanion {
    pub const ALL: [Self; 4] = [Self::Rust, Self::Swift, Self::Kotlin, Self::CSharp];
    const TARGETS: [&'static [NativeTarget]; 4] = [
        &[Macos, Windows, Linux],
        &[Macos, Ios],
        &[Android],
        &[Windows],
    ];
    const SOURCE_NAMES: [&'static str; 4] = ["tac.rs", "tac.swift", "tac.kt", "tac.cs"];
    const LABELS: [&'static str; 4] = ["Rust", "Swift", "Kotlin", "C#"];

    #[must_use]
    pub const fn targets(self) -> &'static [NativeTarget] {
        Self::TARGETS[self as usize]
    }

    #[must_use]
    pub fn supports(self, target: NativeTarget) -> bool {
        self.targets().contains(&target)
    }

    #[must_use]
    pub fn specificity(self, target: NativeTarget) -> Option<u8> {
        self.supports(target)
            .then_some(u8::from(self == Self::Rust))
    }

    #[must_use]
    pub fn most_specific(declared: &[Self], target: NativeTarget) -> Option<Self> {
        declared
            .iter()
            .filter_map(|language| language.specificity(target).map(|rank| (rank, *language)))
            .min_by_key(|(rank, _)| *rank)
            .map(|(_, language)| language)
    }

    #[must_use]
    pub const fn source_name(self) -> &'static str {
        Self::SOURCE_NAMES[self as usize]
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        Self::LABELS[self as usize]
    }

    #[must_use]
    pub fn target_names(self) -> String {
        let names: Vec<&str> = self
            .targets()
            .iter()
            .map(|target| crate::native_target_directory(*target))
            .collect();
        names.join(", ")
    }
}
