use super::Realm;
use super::field_case::FieldCase;
use super::field_name::FieldName;
use super::scan::Messages;
use std::ops::Range;

pub(crate) struct InjectedField {
    pub(crate) at: usize,
    pub(crate) written: String,
    pub(crate) fixed: String,
    case: FieldCase,
    marker: &'static str,
}

impl InjectedField {
    pub(super) const CODE: u16 = 1332;
    const CLASS_MARKER: &'static str = "NeedsAll";
    const FIELD_MARKER: &'static str = "Needs";
    const NAMED_MARKER: &'static str = "Named";
    const TAC_CONTAINER_LANGUAGES: [&'static str; 3] = ["js", "ts", "dart"];
    const PHP_VARIABLE: &'static str = "$";
    const FRAMEWORK_NAMES: [&'static str; 1] = ["behaviors"];

    pub(super) fn on_field(
        language: &str,
        realm: Realm,
        source: &str,
        range: Range<usize>,
    ) -> Vec<Self> {
        FieldName::read(language, realm, source, range)
            .into_iter()
            .filter_map(|field| Self::refused(language, &field, Self::FIELD_MARKER))
            .collect()
    }

    pub(super) fn in_class(
        language: &str,
        realm: Realm,
        source: &str,
        declarations: Vec<Range<usize>>,
    ) -> Vec<Self> {
        declarations
            .into_iter()
            .flat_map(|range| FieldName::read(language, realm, source, range))
            .filter(|field| field.filled)
            .filter(|field| Self::holds_a_class(language, realm, field))
            .filter_map(|field| Self::refused(language, &field, Self::CLASS_MARKER))
            .collect()
    }

    pub(crate) fn message(&self, language: &str) -> String {
        let sigil = Self::sigil(language);
        format!(
            "{} fills `{sigil}{}` with an injected class, so the field uses the {} field case: {}. Write `{sigil}{}` instead. Only the case is checked, so keep your own words, and add {} when the name does not match the provider (ADR 0057)",
            Messages::spelled(language, self.marker),
            self.written,
            Messages::language_name(language),
            self.case.described(),
            self.fixed,
            Messages::spelled(language, Self::NAMED_MARKER),
        )
    }

    fn holds_a_class(language: &str, realm: Realm, field: &FieldName) -> bool {
        let container = Self::TAC_CONTAINER_LANGUAGES.contains(&language);
        realm == Realm::Yon || container && field.names_a_class()
    }

    fn refused(language: &str, field: &FieldName, marker: &'static str) -> Option<Self> {
        let case = FieldCase::of(language, field.public_member)?;
        let framework_name = Self::FRAMEWORK_NAMES.contains(&field.name.as_str());
        let wrong = [case.accepts(&field.name), framework_name] == [false, false];
        wrong.then(|| Self {
            at: field.at,
            written: field.name.clone(),
            fixed: case.spelled(&field.name),
            case,
            marker,
        })
    }

    fn sigil(language: &str) -> &'static str {
        if language == "php" {
            return Self::PHP_VARIABLE;
        }
        ""
    }
}
