use super::injected_store::InjectedStore;

pub(super) struct StoreField {
    indent: String,
    name: String,
    pub(super) type_name: String,
}

impl StoreField {
    const TYPED_PUNCTUATION: [char; 4] = ['!', '?', ';', ','];
    const POINTER: char = '*';
    const SHARED: [&'static str; 2] = ["std::sync::Arc<", "Arc<"];
    const SHARED_END: char = '>';
    const RUBY_READER: &'static str = "attr_reader :";
    const RUBY_RETURN: &'static str = "# @return [";
    const RUBY_TYPE_END: char = ']';

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
            type_name: Self::unshared(
                type_name
                    .trim_start_matches(Self::POINTER)
                    .trim_end_matches(Self::TYPED_PUNCTUATION),
            ),
        })
    }

    pub(super) fn read_ruby(documentation: &[String], line: &str) -> Option<Self> {
        let indent = &line[..line.len() - line.trim_start().len()];
        let name = line.trim().strip_prefix(Self::RUBY_READER)?;
        let type_name = documentation.iter().find_map(|comment| {
            let returned = comment.trim().strip_prefix(Self::RUBY_RETURN)?;
            returned.strip_suffix(Self::RUBY_TYPE_END)
        })?;
        Some(Self {
            indent: indent.to_owned(),
            name: name.to_owned(),
            type_name: type_name.to_owned(),
        })
    }

    fn unshared(type_name: &str) -> String {
        Self::SHARED
            .iter()
            .find_map(|shared| type_name.strip_prefix(shared))
            .and_then(|inner| inner.strip_suffix(Self::SHARED_END))
            .unwrap_or(type_name)
            .to_owned()
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
