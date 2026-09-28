use super::Realm;
use super::field_declaration::FieldDeclaration;
use super::scan::Messages;
use std::ops::Range;

pub(crate) struct NullableField {
    pub(crate) range: Range<usize>,
    pub(crate) fixed: String,
    written: String,
    name: String,
    marker: &'static str,
}

impl NullableField {
    pub(super) const CODE: u16 = 1331;
    const CLASS_MARKER: &'static str = "NeedsAll";
    const FIELD_MARKER: &'static str = "Needs";

    pub(super) fn in_class(
        language: &str,
        realm: Realm,
        source: &str,
        declarations: Vec<Range<usize>>,
    ) -> Vec<Self> {
        declarations
            .into_iter()
            .filter_map(|range| Self::class_field(language, realm, source, range))
            .collect()
    }

    pub(super) fn on_field(language: &str, source: &str, range: Range<usize>) -> Option<Self> {
        let field = FieldDeclaration::read(language, &source[range.clone()])?;
        field
            .is_nullable(language, true)
            .then(|| Self::found(source, range, &field, (language, Self::FIELD_MARKER)))
    }

    pub(crate) fn message(&self, language: &str) -> String {
        format!(
            "{} fills `{}` before any method runs, so it is never null: write `{}` instead of `{}`. A field that may stay empty is plain state: give it a starting value (ADR 0057)",
            Messages::spelled(language, self.marker),
            self.name,
            self.fixed,
            self.written
        )
    }

    fn class_field(
        language: &str,
        realm: Realm,
        source: &str,
        range: Range<usize>,
    ) -> Option<Self> {
        let field = FieldDeclaration::read(language, &source[range.clone()])?;
        let refused = field.is_filled(language, realm) && field.is_nullable(language, false);
        refused.then(|| Self::found(source, range, &field, (language, Self::CLASS_MARKER)))
    }

    fn found(
        source: &str,
        range: Range<usize>,
        field: &FieldDeclaration,
        form: (&str, &'static str),
    ) -> Self {
        let (language, marker) = form;
        let needs = marker == Self::FIELD_MARKER;
        Self {
            written: source[range.clone()]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            range,
            fixed: field.filled_form(language, needs),
            name: field.name.clone(),
            marker,
        }
    }
}
