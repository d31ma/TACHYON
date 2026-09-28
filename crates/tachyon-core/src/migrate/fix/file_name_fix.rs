use super::edits::{Hunk, TextEdit};
use super::renames::{FileMove, RoleRename};
use super::{FileChange, ManualStep};
use crate::annotation_layout::FileNameCheck;
use std::path::Path;

#[derive(Default)]
pub(super) struct FileNameFix {
    pub(super) moves: Vec<FileMove>,
    pub(super) changes: Vec<FileChange>,
    pub(super) manual_steps: Vec<ManualStep>,
}

impl FileNameFix {
    pub(super) fn plan(sources: &[(String, String)], taken: &[String]) -> Self {
        let mut fix = Self::default();
        sources
            .iter()
            .filter(|(relative, _)| !taken.contains(relative))
            .filter_map(|(relative, source)| Self::wanted(relative, source))
            .for_each(|(relative, source, wanted)| {
                fix.add_rename(sources, relative, source, &wanted);
            });
        fix
    }

    pub(super) fn is_allowed_move(from: &str, to: &str) -> bool {
        let from = Path::new(from);
        let to = Path::new(to);
        let checks = [
            from.parent() == to.parent(),
            from.extension() == to.extension(),
            FileNameCheck::is_class_file(from),
        ];
        !checks.contains(&false)
    }

    fn wanted<'s>(relative: &'s str, source: &'s str) -> Option<(&'s str, &'s str, String)> {
        let wanted = FileNameCheck::wanted_name(Path::new(relative), source)?;
        Some((relative, source, wanted))
    }

    fn add_rename(
        &mut self,
        sources: &[(String, String)],
        relative: &str,
        source: &str,
        wanted: &str,
    ) {
        let needle = Self::short_path(relative);
        let referenced = Self::others_mention(sources, relative, &needle);
        let class_free = Self::can_rename_class(sources, relative, wanted);
        self.move_file_when(!referenced, relative, wanted);
        self.rename_class_when(
            [referenced, class_free] == [true, true],
            relative,
            source,
            wanted,
        );
        self.explain_when([referenced, class_free] == [true, false], relative, wanted);
    }

    fn move_file_when(&mut self, wanted_move: bool, relative: &str, wanted: &str) {
        if !wanted_move {
            return;
        }
        self.moves.push(FileMove {
            from: relative.to_owned(),
            to: Self::target(relative, wanted),
        });
    }

    fn rename_class_when(
        &mut self,
        wanted_rename: bool,
        relative: &str,
        source: &str,
        wanted: &str,
    ) {
        if !wanted_rename {
            return;
        }
        let stem = Self::stem(relative);
        let edits: Vec<TextEdit> = RoleRename::word_positions(source, wanted)
            .into_iter()
            .map(|at| TextEdit::replace(at, at + wanted.len(), stem))
            .collect();
        self.changes.push(FileChange {
            source: relative.to_owned(),
            notes: vec![format!(
                "Renamed {wanted} to {stem}, because other files use the name of this file (ADR 0067)."
            )],
            hunks: Hunk::from_edits(source, &edits),
            original: source.to_owned(),
            updated: TextEdit::apply(source, &edits),
        });
    }

    fn explain_when(&mut self, wanted_step: bool, relative: &str, wanted: &str) {
        if !wanted_step {
            return;
        }
        self.manual_steps.push(ManualStep::new(
            relative,
            format!("The file name does not match its class {wanted}, and other files use the file name (ADR 0067)."),
            format!(
                "Rename the file to '{}' and update the files that name it.",
                Self::target(relative, wanted)
            ),
        ));
    }

    fn can_rename_class(sources: &[(String, String)], relative: &str, wanted: &str) -> bool {
        let checks = [
            Self::is_type_name(Self::stem(relative)),
            !Self::others_mention(sources, relative, wanted),
        ];
        !checks.contains(&false)
    }

    fn others_mention(sources: &[(String, String)], relative: &str, word: &str) -> bool {
        sources
            .iter()
            .filter(|(other, _)| other != relative)
            .any(|(_, text)| Self::mentions(text, word))
    }

    fn mentions(text: &str, word: &str) -> bool {
        !RoleRename::word_positions(text, word).is_empty()
    }

    fn is_type_name(stem: &str) -> bool {
        let checks = [
            stem.starts_with(|first: char| first.is_ascii_uppercase()),
            stem.chars().all(Self::is_name_character),
        ];
        !checks.contains(&false)
    }

    fn is_name_character(character: char) -> bool {
        [character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }

    fn stem(relative: &str) -> &str {
        let stem = Path::new(relative).file_stem();
        stem.and_then(|value| value.to_str()).unwrap_or("")
    }

    fn short_path(relative: &str) -> String {
        let parent = Path::new(relative).parent().unwrap_or(Path::new(""));
        let folder = parent.file_name().unwrap_or_default().to_string_lossy();
        format!("{folder}/{}", Self::stem(relative))
    }

    fn target(relative: &str, wanted: &str) -> String {
        let extension = Path::new(relative).extension().unwrap_or_default();
        let file = format!("{wanted}.{}", extension.to_string_lossy());
        let target = Path::new(relative).with_file_name(file);
        target.to_string_lossy().replace('\\', "/")
    }
}
