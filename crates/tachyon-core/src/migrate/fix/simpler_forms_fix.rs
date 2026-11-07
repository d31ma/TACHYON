use super::edits::TextEdit;
use crate::annotation_layout::{SimplerForm, SimplerForms};
use std::path::Path;

pub(super) struct SimplerFormsFix;

impl SimplerFormsFix {
    pub(super) fn plan(root: &Path, relative: &str, source: &str) -> Vec<(TextEdit, String)> {
        SimplerForms::edits(root, relative, source)
            .iter()
            .map(Self::edit)
            .collect()
    }

    fn edit(form: &SimplerForm) -> (TextEdit, String) {
        let note = format!(
            "A simpler form (TY{}): {}",
            form.pattern.code(),
            form.pattern.message()
        );
        (
            TextEdit::replace(form.start, form.end, &form.replacement),
            note,
        )
    }
}
