use super::super::env_names::EnvNames;
use super::super::name_advice::NameAdvice;
use super::super::refused_name::RefusedName;
use super::edits::TextEdit;
use super::{FileChange, ManualStep};
use std::path::Path;

#[derive(Default)]
pub(super) struct EnvExample {
    pub(super) change: Option<FileChange>,
    pub(super) manual_steps: Vec<ManualStep>,
}

impl EnvExample {
    pub(super) fn plan(root: &Path) -> Self {
        let example = EnvNames::read_file(root, EnvNames::EXAMPLE).unwrap_or_default();
        let refused = EnvNames::refused(&example);
        let edits: Vec<(TextEdit, String)> = refused
            .iter()
            .filter_map(|name| Self::edit(&example, name))
            .collect();
        let unnamed = refused
            .iter()
            .filter(|name| name.advice == NameAdvice::Unnamed)
            .map(|name| Self::step(EnvNames::EXAMPLE, name));
        let mut steps: Vec<ManualStep> = unnamed.collect();
        let secrets = EnvNames::read_file(root, EnvNames::SECRETS).unwrap_or_default();
        steps.extend(
            EnvNames::refused(&secrets)
                .iter()
                .map(|name| Self::step(EnvNames::SECRETS, name)),
        );
        let (text_edits, notes): (Vec<TextEdit>, Vec<String>) = edits.into_iter().unzip();
        let change = (!text_edits.is_empty())
            .then(|| FileChange::edited(EnvNames::EXAMPLE, example, &text_edits, notes));
        Self {
            change,
            manual_steps: steps,
        }
    }

    fn edit(source: &str, name: &RefusedName) -> Option<(TextEdit, String)> {
        match &name.advice {
            NameAdvice::Rename(renamed) => Some((
                TextEdit::replace(name.name_start, name.name_end(), renamed),
                format!(
                    "Renamed {} to {renamed}; the server reads it as {} (TY2010).",
                    name.name,
                    EnvNames::config_path(renamed)
                ),
            )),
            NameAdvice::Setting(setting) => Some((
                TextEdit::replace(
                    name.line_start,
                    name.line_end,
                    &format!(
                        "# ty start refuses {} in .env; {setting}.\n# {}",
                        name.name,
                        &source[name.line_start..name.line_end]
                    ),
                ),
                format!(
                    "Commented out {}, which .env cannot set (TY2010); {setting}.",
                    name.name
                ),
            )),
            NameAdvice::Unnamed => None,
        }
    }

    fn step(relative: &str, name: &RefusedName) -> ManualStep {
        ManualStep::new(
            &format!("{relative}:{}", name.line),
            format!(
                "ty start refuses the name '{}' in .env (TY2010).",
                name.name
            ),
            EnvNames::action(relative, name),
        )
    }
}
