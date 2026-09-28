use super::edits::TextEdit;
use crate::annotation_layout::MissingType;

pub(super) struct TypeCommentsFix;

impl TypeCommentsFix {
    pub(super) fn edits(relative: &str, missing: &[MissingType]) -> Vec<(TextEdit, String)> {
        missing
            .iter()
            .filter(|found| found.path == relative)
            .filter_map(Self::edit)
            .collect()
    }

    pub(super) fn steps(relative: &str, missing: &[MissingType]) -> Vec<(String, String)> {
        missing
            .iter()
            .filter(|found| (found.path == relative) & found.insertion.is_none())
            .map(|found| {
                (
                    format!("Line {}: {}", found.line, found.message()),
                    found.help(),
                )
            })
            .collect()
    }

    fn edit(missing: &MissingType) -> Option<(TextEdit, String)> {
        let insertion = missing.insertion.as_ref()?;
        let note = format!(
            "Line {}: added `{}`, the type of a member that had none (TY1356).",
            missing.line,
            insertion.text.trim()
        );
        Some((TextEdit::insert(insertion.at, &insertion.text), note))
    }
}
