use super::injected_store::InjectedStore;

pub(super) struct StoreField {
    indent: String,
    name: String,
    pub(super) type_name: String,
}

impl StoreField {
    const TYPED_PUNCTUATION: [char; 4] = ['!', '?', ';', ','];
    const POINTER: char = '*';

    pub(super) fn read(line: &str, name_first: bool) -> Option<Self> {
        let indent = &line[..line.len() - line.trim_start().len()];
        let words: Vec<&str> = line
            .split_whitespace()
            .skip_while(|word| InjectedStore::MODIFIERS.contains(word))
            .skip_while(|word| InjectedStore::KEYWORDS.contains(word))
            .collect();
        let declaration = words.join(" ");
        let (name, type_name) = declaration
            .split_once(':')
            .map(|(name, type_name)| (name.trim(), type_name.trim()))
            .or_else(|| Self::two_words(&words, name_first))?;
        Some(Self {
            indent: indent.to_owned(),
            name: name.trim_end_matches(Self::TYPED_PUNCTUATION).to_owned(),
            type_name: type_name
                .trim_start_matches(Self::POINTER)
                .trim_end_matches(Self::TYPED_PUNCTUATION)
                .to_owned(),
        })
    }

    fn two_words<'w>(words: &[&'w str], name_first: bool) -> Option<(&'w str, &'w str)> {
        match (words, name_first) {
            ([name, type_name], true) | ([type_name, name], false) => Some((name, type_name)),
            _ => None,
        }
    }

    pub(super) fn filled(&self, form: &str) -> String {
        form.replace("{indent}", &self.indent)
            .replace("{name}", &self.name)
            .replace("{type}", &self.type_name)
    }
}
