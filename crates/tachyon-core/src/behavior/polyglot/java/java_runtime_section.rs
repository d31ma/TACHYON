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

    pub(super) fn joined(self, other: &Self) -> Option<Self> {
        other.entries.iter().try_fold(self, Self::with)
    }

    pub(super) fn text(&self) -> String {
        self.entries.iter().map(|(_, text)| text.as_str()).collect()
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
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::JavaRuntimeSection;

    fn section(entries: &[(&str, &str)]) -> JavaRuntimeSection {
        let mut section = JavaRuntimeSection::default();
        for (label, text) in entries {
            section.add((*label).to_owned(), (*text).to_owned());
        }
        section
    }

    #[test]
    fn equal_entries_of_two_services_are_kept_once() {
        let first = section(&[
            ("Shared", "case \"Shared\": a;"),
            ("Own", "case \"Own\": b;"),
        ]);
        let second = section(&[
            ("Shared", "case \"Shared\": a;"),
            ("Other", "case \"Other\": c;"),
        ]);
        let joined = first.joined(&second).expect("no conflict");
        assert_eq!(
            joined.text(),
            "case \"Shared\": a;case \"Own\": b;case \"Other\": c;"
        );
        assert_eq!(
            joined.labels().collect::<Vec<_>>(),
            ["Shared", "Own", "Other"]
        );
    }

    #[test]
    fn one_label_with_two_texts_cannot_join() {
        let first = section(&[("Input", "case \"Input\": a;")]);
        let second = section(&[("Input", "case \"Input\": b;")]);
        assert!(first.joined(&second).is_none());
    }

    #[test]
    fn an_empty_entry_is_not_kept() {
        assert_eq!(section(&[("Plain", "")]).labels().count(), 0);
    }
}
