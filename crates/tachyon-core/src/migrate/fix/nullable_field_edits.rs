use super::edits::TextEdit;
use crate::annotation_rules::AnnotationRules;
use std::path::Path;

pub(super) struct NullableFieldEdits;

impl NullableFieldEdits {
    pub(super) fn plan(relative: &str, source: &str) -> Vec<(TextEdit, String)> {
        AnnotationRules::nullable_fields(Path::new(relative), source)
            .into_iter()
            .filter(|field| !source[field.range.clone()].contains('\n'))
            .map(|field| {
                (
                    TextEdit::replace(field.range.start, field.range.end, &field.fixed),
                    Self::note(source, field.range.start, &field.fixed),
                )
            })
            .collect()
    }

    fn note(source: &str, at: usize, fixed: &str) -> String {
        let line = source[..at].matches('\n').count() + 1;
        format!(
            "Declared `{fixed}` on line {line}: Tachyon fills this field before any method runs, so it is never null (ADR 0057)."
        )
    }
}
