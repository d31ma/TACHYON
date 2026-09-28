use super::Realm;
use super::field_declaration::FieldDeclaration;
use std::ops::Range;

pub(super) struct FieldName {
    pub(super) name: String,
    pub(super) at: usize,
    pub(super) public_member: bool,
    pub(super) filled: bool,
    type_text: String,
}

impl FieldName {
    const TYPE_FIRST: [&'static str; 5] = ["dart", "cs", "java", "groovy", "php"];
    const RUBY_READER: &'static str = "attr_reader";
    const SYMBOL_MARK: char = ':';
    const DECLARED_ENDS: [char; 2] = ['=', '{'];
    const BUILT_IN_TYPES: [&'static str; 24] = [
        "String",
        "Number",
        "Boolean",
        "Object",
        "Array",
        "ReadonlyArray",
        "Map",
        "Set",
        "Date",
        "Record",
        "Promise",
        "Function",
        "Symbol",
        "BigInt",
        "RegExp",
        "Error",
        "Partial",
        "Readonly",
        "List",
        "Iterable",
        "DateTime",
        "Duration",
        "Uri",
        "Future",
    ];

    pub(super) fn read(
        language: &str,
        realm: Realm,
        source: &str,
        range: Range<usize>,
    ) -> Vec<Self> {
        Self::names(language, realm, source, range)
    }

    pub(super) fn names_a_class(&self) -> bool {
        let written = Self::first_word(self.class_text());
        let uppercase = written.starts_with(|letter: char| letter.is_ascii_uppercase());
        let built_in = Self::BUILT_IN_TYPES.contains(&written);
        [uppercase, built_in] == [true, false]
    }

    fn names(language: &str, realm: Realm, source: &str, range: Range<usize>) -> Vec<Self> {
        match language {
            "rb" => Self::ruby_names(source, range),
            "js" => Self::script_name(source, range).into_iter().collect(),
            "go" => Self::go_name(source, range).into_iter().collect(),
            _ => Self::declared_name(language, realm, source, range)
                .into_iter()
                .collect(),
        }
    }

    fn ruby_names(source: &str, range: Range<usize>) -> Vec<Self> {
        let text = &source[range.clone()];
        let symbols = text
            .trim_start()
            .strip_prefix(Self::RUBY_READER)
            .unwrap_or_default();
        let start = range.start + text.len() - symbols.len();
        symbols
            .match_indices(Self::SYMBOL_MARK)
            .map(|(offset, _)| Self::ruby_symbol(symbols, offset + 1, start))
            .filter(|field| !field.name.is_empty())
            .collect()
    }

    fn ruby_symbol(symbols: &str, offset: usize, start: usize) -> Self {
        let name = Self::first_word(&symbols[offset..]).to_owned();
        Self::plain(name, start + offset)
    }

    fn script_name(source: &str, range: Range<usize>) -> Option<Self> {
        let text = &source[range.clone()];
        let declared = text
            .split('=')
            .next()?
            .trim()
            .trim_end_matches(';')
            .trim_end();
        let offset = text.find(declared)?;
        let mut field = Self::plain(declared.to_owned(), range.start + offset);
        field.filled = !text.contains('=');
        Self::is_identifier(declared).then_some(field)
    }

    fn go_name(source: &str, range: Range<usize>) -> Option<Self> {
        let text = &source[range.clone()];
        let mut words = text.split_whitespace();
        let name = words.next()?;
        let type_text = words.next()?;
        let offset = text.find(name)?;
        let mut field = Self::plain(name.to_owned(), range.start + offset);
        type_text.clone_into(&mut field.type_text);
        Self::is_identifier(name).then_some(field)
    }

    fn declared_name(
        language: &str,
        realm: Realm,
        source: &str,
        range: Range<usize>,
    ) -> Option<Self> {
        let text = &source[range.clone()];
        let field = FieldDeclaration::read(language, text)?;
        let offset = Self::name_offset(language, text, &field.name)?;
        Some(Self {
            name: field.name.clone(),
            at: range.start + offset,
            public_member: field.is_public_member(),
            filled: field.is_filled(language, realm),
            type_text: field.type_text().to_owned(),
        })
    }

    fn plain(name: String, at: usize) -> Self {
        Self {
            name,
            at,
            public_member: false,
            filled: true,
            type_text: String::new(),
        }
    }

    fn name_offset(language: &str, text: &str, name: &str) -> Option<usize> {
        let declared = text.split(Self::DECLARED_ENDS).next().unwrap_or(text);
        let offsets = declared
            .match_indices(name)
            .map(|(offset, _)| offset)
            .filter(|offset| Self::is_whole_word(declared, *offset, name.len()))
            .collect::<Vec<_>>();
        Self::chosen(language, &offsets)
    }

    fn chosen(language: &str, offsets: &[usize]) -> Option<usize> {
        if Self::TYPE_FIRST.contains(&language) {
            return offsets.last().copied();
        }
        offsets.first().copied()
    }

    fn is_whole_word(text: &str, offset: usize, length: usize) -> bool {
        let before = text[..offset].chars().next_back();
        let after = text[offset + length..].chars().next();
        [before, after]
            .into_iter()
            .flatten()
            .all(|letter| !Self::is_word_character(letter))
    }

    fn is_identifier(text: &str) -> bool {
        let starts_with_digit = text.starts_with(|letter: char| letter.is_ascii_digit());
        let word = text.chars().all(Self::is_word_character);
        [text.is_empty(), starts_with_digit, word] == [false, false, true]
    }

    fn is_word_character(letter: char) -> bool {
        letter.is_alphanumeric() || letter == '_'
    }

    fn first_word(text: &str) -> &str {
        let end = text
            .find(|letter: char| !Self::is_word_character(letter))
            .unwrap_or(text.len());
        &text[..end]
    }

    fn class_text(&self) -> &str {
        [self.type_text.as_str(), self.name.as_str()]
            .into_iter()
            .find(|text| !text.is_empty())
            .unwrap_or_default()
    }
}
