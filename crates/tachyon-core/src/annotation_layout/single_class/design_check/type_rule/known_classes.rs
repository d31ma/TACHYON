use super::super::super::duplication::ProjectSource;
use super::super::class_file::ClassFile;
use super::super::name_forms::NameForms;
use super::super::role_layer::RoleLayer;
use std::collections::{BTreeSet, HashMap};

pub(super) struct KnownClasses {
    names: BTreeSet<String>,
    fields: HashMap<String, String>,
}

impl KnownClasses {
    const PRIVATE_MARK: char = '_';
    const SEPARATOR: char = '/';
    const EXTENSION_MARK: char = '.';
    const FRAMEWORK_FIELDS: [(&'static str, &'static str); 2] = [
        ("applicationEventPublisher", "ApplicationEventPublisher"),
        ("application_event_publisher", "ApplicationEventPublisher"),
    ];

    pub(super) fn read(classes: &[ClassFile], sources: &[ProjectSource]) -> Self {
        Self {
            names: sources
                .iter()
                .filter_map(|source| Self::class_name(&source.path))
                .collect(),
            fields: classes
                .iter()
                .filter(|class| RoleLayer::of(class.folder).is_injected())
                .flat_map(Self::field_forms)
                .chain(Self::framework_fields())
                .collect(),
        }
    }

    pub(super) fn is_framework_field(field: &str) -> bool {
        Self::FRAMEWORK_FIELDS
            .iter()
            .any(|(name, _)| *name == field)
    }

    pub(super) fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub(super) fn injected(&self, field: &str) -> Option<&str> {
        self.fields
            .get(field.trim_start_matches(Self::PRIVATE_MARK))
            .map(String::as_str)
    }

    fn class_name(path: &str) -> Option<String> {
        let file = path.rsplit(Self::SEPARATOR).next()?;
        let stem = file.split(Self::EXTENSION_MARK).next()?;
        let named = stem.starts_with(char::is_uppercase);
        named.then(|| stem.to_owned())
    }

    fn framework_fields() -> impl Iterator<Item = (String, String)> {
        Self::FRAMEWORK_FIELDS
            .iter()
            .map(|(field, class)| ((*field).to_owned(), (*class).to_owned()))
    }

    fn field_forms(class: &ClassFile) -> [(String, String); 3] {
        NameForms::of(class.name).map(|form| (form, class.name.to_owned()))
    }
}
