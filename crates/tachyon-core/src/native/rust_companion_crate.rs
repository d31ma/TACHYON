use super::rust_dependency::RustDependency;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

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
    const PLATFORMS: [(&'static str, &'static str); 3] = [
        ("linux", "cfg(target_os = \"linux\")"),
        ("macos", "cfg(target_os = \"macos\")"),
        ("windows", "cfg(windows)"),
    ];
    const MAX_EXECUTABLES: usize = 8;
    const MAX_EXECUTABLE_NAME: usize = 64;
    const MAX_DEPENDENCIES: usize = 64;
    const SOURCE_EXTENSION: &'static str = ".rs";

    pub(super) fn uses_cargo(&self) -> bool {
        self.has_dependencies() || !self.executables.is_empty()
    }

    pub(super) fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
            || self
                .platform_dependencies
                .values()
                .any(|dependencies| !dependencies.is_empty())
    }

    pub(super) fn problem(&self) -> Option<String> {
        Self::dependency_problem("dependencies", &self.dependencies)
            .or_else(|| self.platform_problem())
            .or_else(|| self.executable_problem())
    }

    pub(super) fn entry_class(entry: &str) -> Option<&str> {
        let file = entry.rsplit('/').next()?;
        let class = file.strip_suffix(Self::SOURCE_EXTENSION)?;
        let mut characters = class.chars();
        let starts_upper = characters
            .next()
            .is_some_and(|first| first.is_ascii_uppercase());
        (starts_upper && characters.all(|character| character.is_ascii_alphanumeric()))
            .then_some(class)
    }

    pub(super) fn manifest(
        &self,
        library: &Path,
        crate_types: &[&str],
        binaries: &[(String, std::path::PathBuf)],
    ) -> String {
        let types = crate_types
            .iter()
            .map(|kind| format!("\"{kind}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let mut manifest = format!(
            "[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
             [workspace]\n\n[lib]\nname = \"{}\"\npath = {}\ncrate-type = [{types}]\n",
            Self::PACKAGE,
            Self::LIBRARY,
            Self::toml_path(library)
        );
        for (name, path) in binaries {
            let _ = write!(
                manifest,
                "\n[[bin]]\nname = \"{name}\"\npath = {}\n",
                Self::toml_path(path)
            );
        }
        manifest.push_str(&Self::dependency_table("dependencies", &self.dependencies));
        for (platform, condition) in Self::PLATFORMS {
            if let Some(dependencies) = self.platform_dependencies.get(platform) {
                manifest.push_str(&Self::dependency_table(
                    &format!("target.'{condition}'.dependencies"),
                    dependencies,
                ));
            }
        }
        manifest
    }

    fn dependency_table(header: &str, dependencies: &BTreeMap<String, RustDependency>) -> String {
        if dependencies.is_empty() {
            return String::new();
        }
        let mut table = format!("\n[{header}]\n");
        for (name, dependency) in dependencies {
            let _ = writeln!(table, "{name} = {}", dependency.toml());
        }
        table
    }

    pub(super) fn toml_path(path: &Path) -> String {
        let text = path.to_string_lossy();
        format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
    }

    fn dependency_problem(
        place: &str,
        dependencies: &BTreeMap<String, RustDependency>,
    ) -> Option<String> {
        if dependencies.len() > Self::MAX_DEPENDENCIES {
            return Some(format!(
                "tac.application.rust.{place} lists more than {} crates.",
                Self::MAX_DEPENDENCIES
            ));
        }
        dependencies.iter().find_map(|(name, dependency)| {
            if !RustDependency::valid_name(name) || name == Self::PACKAGE {
                return Some(format!(
                    "tac.application.rust.{place} names \"{name}\", which is not a crate name."
                ));
            }
            dependency
                .problem()
                .map(|problem| format!("tac.application.rust.{place}.{name}: the {problem}."))
        })
    }

    fn platform_problem(&self) -> Option<String> {
        self.platform_dependencies
            .iter()
            .find_map(|(platform, dependencies)| {
                if !Self::PLATFORMS.iter().any(|(known, _)| known == platform) {
                    return Some(format!(
                        "tac.application.rust.platformDependencies names \"{platform}\"; use linux, macos or windows."
                    ));
                }
                Self::dependency_problem(&format!("platformDependencies.{platform}"), dependencies)
            })
    }

    fn executable_problem(&self) -> Option<String> {
        if self.executables.len() > Self::MAX_EXECUTABLES {
            return Some(format!(
                "tac.application.rust.executables lists more than {} executables.",
                Self::MAX_EXECUTABLES
            ));
        }
        self.executables.iter().find_map(|(name, entry)| {
            if !Self::valid_executable_name(name) {
                return Some(format!(
                    "tac.application.rust.executables names \"{name}\"; an executable name uses lowercase letters, digits and '-' and starts with a letter."
                ));
            }
            (!Self::valid_entry(entry)).then(|| {
                format!(
                    "tac.application.rust.executables.{name} is \"{entry}\"; point it at a client Rust file such as /shared/native/EventEmitterMain.rs, whose class has `pub fn run() -> i32`."
                )
            })
        })
    }

    fn valid_executable_name(name: &str) -> bool {
        name.len() <= Self::MAX_EXECUTABLE_NAME
            && name != Self::PACKAGE
            && name.starts_with(|first: char| first.is_ascii_lowercase())
            && name.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
            })
    }

    fn valid_entry(entry: &str) -> bool {
        entry.starts_with('/')
            && !entry.contains('\\')
            && entry
                .split('/')
                .skip(1)
                .all(|part| !part.is_empty() && part != "." && part != "..")
            && Self::entry_class(entry).is_some()
    }
}
