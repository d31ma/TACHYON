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
    const FILE_EXTENSIONS: [&'static str; 4] = ["rs", "swift", "kt", "cs"];
    const LABELS: [&'static str; 4] = ["Rust", "Swift", "Kotlin", "C#"];
    const PLATFORM_ONLY: [Self; 1] = [Self::Swift];

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

    pub(crate) const fn file_extension(self) -> &'static str {
        Self::FILE_EXTENSIONS[self as usize]
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

    pub(crate) fn unreachable(
        route: &str,
        class: &str,
        declared: &[Self],
        target_name: &str,
        target: Option<NativeTarget>,
    ) -> crate::Failure {
        let declared_names: Vec<String> = declared
            .iter()
            .map(|language| format!("{} ({})", language.label(), language.target_names()))
            .collect();
        let listed = declared_names.join(", ");
        let native = Self::platform_alternatives(class, target);
        crate::Failure::one(crate::failure::diagnostic(
            1010,
            format!(
                "Route '{route}' has no companion that runs on {target_name}. It declares {listed}."
            ),
            Some(format!(
                "Swift links only on Apple platforms. JavaScript, TypeScript, Dart, Rust, C#, \
                 Go and Kotlin reach every target. Add client/services/{class}.js, .ts, \
                 .dart, .rs, .cs, .go or .kt{native} to give this service behaviour on {target_name}, \
                 or build a target the declared language supports.",
            )),
            None,
        ))
    }

    fn platform_alternatives(class: &str, target: Option<NativeTarget>) -> String {
        let reaching = target
            .map(|target| Self::reaching_files(class, target))
            .unwrap_or_default();
        Self::alternatives_text(&reaching)
    }

    fn reaching_files(class: &str, target: NativeTarget) -> Vec<String> {
        Self::PLATFORM_ONLY
            .into_iter()
            .filter(|language| language.supports(target))
            .map(|language| format!("{class}.{}", language.file_extension()))
            .collect()
    }

    fn alternatives_text(reaching: &[String]) -> String {
        if reaching.is_empty() {
            return String::new();
        }
        format!(", or {}", reaching.join(" or "))
    }
}
