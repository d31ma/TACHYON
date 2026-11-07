use super::rust_dependency::RustDependency;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct RustCompanionCrate {
    #[serde(default)]
    pub(super) dependencies: BTreeMap<String, RustDependency>,
    #[serde(default)]
    pub(super) platform_dependencies: BTreeMap<String, BTreeMap<String, RustDependency>>,
    #[serde(default)]
    pub(super) executables: BTreeMap<String, String>,
}

impl RustCompanionCrate {
    pub(super) const PACKAGE: &'static str = "tachyon-companion";
    pub(super) const LIBRARY: &'static str = "tachyon_companion";
    const PLATFORMS: [(&'static str, &'static str); 5] = [
        ("android", "cfg(target_os = \"android\")"),
        ("ios", "cfg(target_os = \"ios\")"),
        ("linux", "cfg(target_os = \"linux\")"),
        ("macos", "cfg(target_os = \"macos\")"),
        ("windows", "cfg(windows)"),
    ];
    const DEPENDENCIES: &'static str = "dependencies";
    const MAX_EXECUTABLES: usize = 8;
    const MAX_EXECUTABLE_NAME: usize = 64;
    const MAX_DEPENDENCIES: usize = 64;
    const SOURCE_EXTENSION: &'static str = ".rs";
    const PATH_SEPARATOR: char = '/';
    const NAME_MARK: char = '-';
    const CURRENT_FOLDER: &'static str = ".";
    const PARENT_FOLDER: &'static str = "..";

    pub(super) fn uses_cargo(&self) -> bool {
        self.has_dependencies() | !self.executables.is_empty()
    }

    pub(super) fn has_dependencies(&self) -> bool {
        let platform = self
            .platform_dependencies
            .values()
            .any(|dependencies| !dependencies.is_empty());
        !self.dependencies.is_empty() | platform
    }

    pub(super) fn problem(&self) -> Option<String> {
        Self::dependency_problem(Self::DEPENDENCIES, &self.dependencies)
            .or_else(|| self.platform_problem())
            .or_else(|| self.executable_problem())
    }

    pub(super) fn entry_class(entry: &str) -> Option<&str> {
        let file = entry.rsplit(Self::PATH_SEPARATOR).next()?;
        let class = file.strip_suffix(Self::SOURCE_EXTENSION)?;
        let starts_upper = class.starts_with(|first: char| first.is_ascii_uppercase());
        let plain = class
            .chars()
            .all(|character| character.is_ascii_alphanumeric());
        (starts_upper & plain).then_some(class)
    }

    pub(super) fn manifest(
        &self,
        library: &Path,
        crate_types: &[&str],
        binaries: &[(String, PathBuf)],
    ) -> String {
        let types = crate_types
            .iter()
            .map(|kind| format!("\"{kind}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let package = format!(
            "[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
             [workspace]\n\n[lib]\nname = \"{}\"\npath = {}\ncrate-type = [{types}]\n",
            Self::PACKAGE,
            Self::LIBRARY,
            Self::toml_path(library)
        );
        let binaries = binaries
            .iter()
            .fold(String::new(), |mut text, (name, path)| {
                let _ = write!(
                    text,
                    "\n[[bin]]\nname = \"{name}\"\npath = {}\n",
                    Self::toml_path(path)
                );
                text
            });
        let platforms = Self::PLATFORMS
            .iter()
            .filter_map(|(platform, condition)| self.platform_table(platform, condition))
            .collect::<String>();
        let common = Self::dependency_table(Self::DEPENDENCIES, &self.dependencies);
        format!("{package}{binaries}{common}{platforms}")
    }

    pub(super) fn toml_path(path: &Path) -> String {
        let text = path.to_string_lossy();
        format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
    }

    fn platform_table(&self, platform: &str, condition: &str) -> Option<String> {
        self.platform_dependencies
            .get(platform)
            .map(|dependencies| {
                Self::dependency_table(&format!("target.'{condition}'.dependencies"), dependencies)
            })
    }

    fn dependency_table(header: &str, dependencies: &BTreeMap<String, RustDependency>) -> String {
        let lines = dependencies
            .iter()
            .fold(String::new(), |mut text, (name, dependency)| {
                let _ = writeln!(text, "{name} = {}", dependency.toml());
                text
            });
        let table: Option<String> = (!lines.is_empty()).then(|| format!("\n[{header}]\n{lines}"));
        table.unwrap_or_default()
    }

    fn dependency_problem(
        place: &str,
        dependencies: &BTreeMap<String, RustDependency>,
    ) -> Option<String> {
        Self::dependency_count_problem(place, dependencies.len()).or_else(|| {
            dependencies
                .iter()
                .find_map(|(name, dependency)| Self::named_problem(place, name, dependency))
        })
    }

    fn dependency_count_problem(place: &str, count: usize) -> Option<String> {
        (count > Self::MAX_DEPENDENCIES).then(|| {
            format!(
                "tac.application.rust.{place} lists more than {} crates.",
                Self::MAX_DEPENDENCIES
            )
        })
    }

    fn named_problem(place: &str, name: &str, dependency: &RustDependency) -> Option<String> {
        Self::crate_name_problem(place, name).or_else(|| {
            dependency
                .problem()
                .map(|problem| format!("tac.application.rust.{place}.{name}: the {problem}."))
        })
    }

    fn crate_name_problem(place: &str, name: &str) -> Option<String> {
        let refused = [!RustDependency::valid_name(name), name == Self::PACKAGE].contains(&true);
        refused.then(|| {
            format!("tac.application.rust.{place} names \"{name}\", which is not a crate name.")
        })
    }

    fn platform_problem(&self) -> Option<String> {
        self.platform_dependencies
            .iter()
            .find_map(|(platform, dependencies)| Self::one_platform_problem(platform, dependencies))
    }

    fn one_platform_problem(
        platform: &str,
        dependencies: &BTreeMap<String, RustDependency>,
    ) -> Option<String> {
        let known = Self::PLATFORMS.iter().any(|(known, _)| *known == platform);
        (!known)
            .then(|| {
                format!(
                    "tac.application.rust.platformDependencies names \"{platform}\"; use android, ios, linux, macos or windows."
                )
            })
            .or_else(|| {
                Self::dependency_problem(&format!("platformDependencies.{platform}"), dependencies)
            })
    }

    fn executable_problem(&self) -> Option<String> {
        self.executable_count_problem().or_else(|| {
            self.executables
                .iter()
                .find_map(|(name, entry)| Self::one_executable_problem(name, entry))
        })
    }

    fn executable_count_problem(&self) -> Option<String> {
        (self.executables.len() > Self::MAX_EXECUTABLES).then(|| {
            format!(
                "tac.application.rust.executables lists more than {} executables.",
                Self::MAX_EXECUTABLES
            )
        })
    }

    fn one_executable_problem(name: &str, entry: &str) -> Option<String> {
        (!Self::valid_executable_name(name))
            .then(|| {
                format!(
                    "tac.application.rust.executables names \"{name}\"; an executable name uses lowercase letters, digits and '-' and starts with a letter."
                )
            })
            .or_else(|| Self::entry_problem(name, entry))
    }

    fn entry_problem(name: &str, entry: &str) -> Option<String> {
        (!Self::valid_entry(entry)).then(|| {
            format!(
                "tac.application.rust.executables.{name} is \"{entry}\"; point it at a client Rust file such as /shared/native/EventEmitterMain.rs, whose class has `pub fn run() -> i32`."
            )
        })
    }

    fn valid_executable_name(name: &str) -> bool {
        let bounded = name.len() <= Self::MAX_EXECUTABLE_NAME;
        let starts_with_letter = name.starts_with(|first: char| first.is_ascii_lowercase());
        let plain = name.chars().all(Self::executable_character);
        [bounded, starts_with_letter, plain, name != Self::PACKAGE] == [true; 4]
    }

    fn executable_character(character: char) -> bool {
        [
            character.is_ascii_lowercase(),
            character.is_ascii_digit(),
            character == Self::NAME_MARK,
        ]
        .contains(&true)
    }

    fn valid_entry(entry: &str) -> bool {
        let rooted = entry.starts_with(Self::PATH_SEPARATOR) & !entry.contains('\\');
        let parts = entry
            .split(Self::PATH_SEPARATOR)
            .skip(1)
            .all(Self::plain_part);
        rooted & parts & Self::entry_class(entry).is_some()
    }

    fn plain_part(part: &str) -> bool {
        [
            !part.is_empty(),
            part != Self::CURRENT_FOLDER,
            part != Self::PARENT_FOLDER,
        ] == [true; 3]
    }
}
