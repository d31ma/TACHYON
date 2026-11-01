use super::name_advice::NameAdvice;
use super::refused_name::RefusedName;
use super::{MigrationFinding, MigrationStatus};
use std::fs;
use std::path::Path;

pub(super) struct EnvNames;

impl EnvNames {
    pub(super) const SECRETS: &'static str = ".env";
    pub(super) const EXAMPLE: &'static str = ".env.example";
    const MAX_BYTES: u64 = 1_048_576;
    const READS: [&'static str; 2] = ["copied to .env, ty start would refuse", "ty start refuses"];

    pub(super) fn read_file(root: &Path, relative: &str) -> Option<String> {
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).ok()?;
        let small = metadata.len() <= Self::MAX_BYTES;
        let readable = [metadata.is_file(), metadata.is_symlink(), small] == [true, false, true];
        readable.then_some(())?;
        fs::read_to_string(path).ok()
    }

    pub(super) fn refused(source: &str) -> Vec<RefusedName> {
        let starts = source.split_inclusive('\n').scan(0, |offset, raw: &str| {
            let start = *offset;
            *offset += raw.len();
            Some(start)
        });
        source
            .split_inclusive('\n')
            .zip(starts)
            .enumerate()
            .filter_map(|(index, (raw, start))| RefusedName::read(raw, start, index + 1))
            .collect()
    }

    pub(super) fn config_path(name: &str) -> String {
        let segments: Vec<String> = name.split('_').map(str::to_ascii_lowercase).collect();
        segments.join(".")
    }

    pub(super) fn findings(root: &Path) -> Vec<MigrationFinding> {
        [Self::SECRETS, Self::EXAMPLE]
            .into_iter()
            .flat_map(|relative| Self::file_findings(root, relative))
            .collect()
    }

    pub(super) fn action(relative: &str, refused: &RefusedName) -> String {
        Self::advised_action(relative == Self::EXAMPLE, &refused.advice)
    }

    fn file_findings(root: &Path, relative: &str) -> Vec<MigrationFinding> {
        let source = Self::read_file(root, relative).unwrap_or_default();
        Self::refused(&source)
            .iter()
            .map(|refused| Self::finding(relative, refused))
            .collect()
    }

    fn finding(relative: &str, refused: &RefusedName) -> MigrationFinding {
        let read = Self::READS[usize::from(relative == Self::SECRETS)];
        let detail = format!(
            "{read} the name '{}' (TY2010): {}.",
            refused.name,
            Self::reason(&refused.advice)
        );
        MigrationFinding::new(
            &format!("{relative}:{}", refused.line),
            "environment.env_name",
            MigrationStatus::Changed,
            &detail,
            Some(&Self::action(relative, refused)),
        )
    }

    fn reason(advice: &NameAdvice) -> &'static str {
        match advice {
            NameAdvice::Setting(_) => {
                "it names a framework transport, profile or runtime setting, which .env cannot set"
            }
            NameAdvice::Rename(_) | NameAdvice::Unnamed => {
                ".env holds only server secrets with uppercase YON_ names"
            }
        }
    }

    fn advised_action(fixed: bool, advice: &NameAdvice) -> String {
        match advice {
            NameAdvice::Rename(name) if fixed => format!(
                "Run 'ty migrate fix --write' to rename it to {name}; the server reads it as {}.",
                Self::config_path(name)
            ),
            NameAdvice::Rename(name) => format!(
                "Rename it to {name} in .env and keep its value; the server reads it as {}.",
                Self::config_path(name)
            ),
            NameAdvice::Setting(setting) if fixed => {
                format!("Run 'ty migrate fix --write' to comment it out, then {setting}.")
            }
            NameAdvice::Setting(setting) => format!("Remove it from .env, then {setting}."),
            NameAdvice::Unnamed => String::from(
                "Rename it to an uppercase YON_ name of at most 16 underscore-separated segments, each starting with a letter.",
            ),
        }
    }
}
