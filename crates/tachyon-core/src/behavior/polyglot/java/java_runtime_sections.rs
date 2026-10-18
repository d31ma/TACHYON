#[derive(Clone, Default)]
pub(crate) struct JavaRuntimeSection {
    entries: Vec<(String, String)>,
}

impl JavaRuntimeSection {
    pub(super) fn add(&mut self, label: String, text: String) {
        self.entries
            .extend((!text.is_empty()).then_some((label, text)));
    }

    pub(super) fn labels(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(label, _)| label.as_str())
    }

    fn joined(self, other: &Self) -> Option<Self> {
        other.entries.iter().try_fold(self, Self::with)
    }

    fn with(mut self, (label, text): &(String, String)) -> Option<Self> {
        let same = self
            .entries
            .iter()
            .find(|(known, _)| known == label)
            .map(|(_, known)| known == text);
        if let Some(same) = same {
            return same.then_some(self);
        }
        self.entries.push((label.clone(), text.clone()));
        Some(self)
    }

    fn text(&self) -> String {
        self.entries.iter().map(|(_, text)| text.as_str()).collect()
    }
}

#[derive(Clone, Default)]
pub(crate) struct JavaRuntimeSections {
    pub(super) configurations: JavaRuntimeSection,
    pub(super) entities: JavaRuntimeSection,
    pub(super) evictions: JavaRuntimeSection,
    pub(super) properties: JavaRuntimeSection,
    pub(super) values: JavaRuntimeSection,
    pub(super) selections: JavaRuntimeSection,
    pub(super) validations: JavaRuntimeSection,
}

impl JavaRuntimeSections {
    pub(crate) fn joined(self, other: &Self) -> Option<Self> {
        Some(Self {
            configurations: self.configurations.joined(&other.configurations)?,
            entities: self.entities.joined(&other.entities)?,
            evictions: self.evictions.joined(&other.evictions)?,
            properties: self.properties.joined(&other.properties)?,
            values: self.values.joined(&other.values)?,
            selections: self.selections.joined(&other.selections)?,
            validations: self.validations.joined(&other.validations)?,
        })
    }

    pub(crate) fn configured(&self) -> impl Iterator<Item = &str> {
        self.configurations.labels()
    }

    pub(super) fn fill_runtime(&self, module_id: &str, environment: &str) -> String {
        include_str!("../java_runtime.java")
            .replace("__MODULE__", module_id)
            .replace("__ENVIRONMENT__", environment)
            .replace("__CONFIGURATIONS__", &self.configurations.text())
            .replace("__ENTITIES__", &self.entities.text())
            .replace("__EVICTIONS__", &self.evictions.text())
            .replace("__PROPERTIES__", &self.properties.text())
            .replace("__VALUES__", &self.values.text())
            .replace("__SELECTIONS__", &self.selections.text())
            .replace("__VALIDATIONS__", &self.validations.text())
    }
}

#[derive(Clone)]
pub(crate) struct JavaPageRuntime {
    pub(super) module_id: String,
    pub(super) sections: JavaRuntimeSections,
}

impl JavaPageRuntime {
    pub(crate) fn new(module_id: String, sections: JavaRuntimeSections) -> Self {
        Self {
            module_id,
            sections,
        }
    }

    pub(crate) fn configured(&self) -> impl Iterator<Item = &str> {
        self.sections.configured()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{JavaRuntimeSection, JavaRuntimeSections};

    fn configured(entries: &[(&str, &str)]) -> JavaRuntimeSections {
        let mut sections = JavaRuntimeSections::default();
        for (label, text) in entries {
            sections
                .configurations
                .add((*label).to_owned(), (*text).to_owned());
        }
        sections
    }

    #[test]
    fn equal_entries_of_two_services_are_kept_once() {
        let first = configured(&[
            ("Shared", "case \"Shared\": a;"),
            ("Own", "case \"Own\": b;"),
        ]);
        let second = configured(&[
            ("Shared", "case \"Shared\": a;"),
            ("Other", "case \"Other\": c;"),
        ]);
        let joined = first.joined(&second).expect("no conflict");
        assert_eq!(
            joined.configurations.text(),
            "case \"Shared\": a;case \"Own\": b;case \"Other\": c;"
        );
        assert_eq!(
            joined.configured().collect::<Vec<_>>(),
            ["Shared", "Own", "Other"]
        );
    }

    #[test]
    fn one_label_with_two_texts_cannot_join() {
        let first = configured(&[("Input", "case \"Input\": a;")]);
        let second = configured(&[("Input", "case \"Input\": b;")]);
        assert!(first.joined(&second).is_none());
    }

    #[test]
    fn an_empty_entry_is_not_kept() {
        let mut section = JavaRuntimeSection::default();
        section.add("Plain".to_owned(), String::new());
        assert_eq!(section.labels().count(), 0);
    }
}
