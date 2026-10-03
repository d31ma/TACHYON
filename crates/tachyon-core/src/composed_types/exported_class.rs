use crate::annotation_layout::{brace_depth_after, masked_source};
use std::path::Path;

pub(super) struct ExportedClass {
    pub(super) name: String,
    pub(super) member: String,
}

impl ExportedClass {
    pub(super) fn find(path: &Path, source: &str) -> Vec<Self> {
        masked_source(path, source)
            .map(|code| Self::find_classes(path, source, &code))
            .unwrap_or_default()
    }

    fn find_classes(path: &Path, source: &str, code: &str) -> Vec<Self> {
        crate::annotation_layout::UtilityLayout::is_utility(path)
            .then_some(path)
            .map_or_else(
                || Self::find_in_code(code),
                |path| Self::utility_class(path, source, code),
            )
    }

    fn utility_class(path: &Path, source: &str, code: &str) -> Vec<Self> {
        crate::annotation_layout::UtilityLayout::check(path, source)
            .ok()
            .and_then(|()| path.file_stem())
            .and_then(|name| name.to_str())
            .map(|name| Self {
                name: name.to_owned(),
                member: Self::utility_member(name, code),
            })
            .into_iter()
            .collect()
    }

    fn utility_member(name: &str, code: &str) -> String {
        Self::find_in_code(code)
            .into_iter()
            .find(|class| class.name == name)
            .map_or_else(|| name.to_owned(), |class| class.member)
    }

    fn find_in_code(code: &str) -> Vec<Self> {
        let mut classes = Vec::new();
        let mut depth = 0_usize;
        for line in code.lines() {
            classes.extend(Self::top_level_class(line, depth));
            depth = brace_depth_after(line, depth);
        }
        classes
    }

    fn top_level_class(line: &str, depth: usize) -> Option<Self> {
        (depth == 0).then(|| Self::read(line)).flatten()
    }

    fn read(line: &str) -> Option<Self> {
        let mut words = line.split_whitespace().peekable();
        words.next().filter(|word| *word == "export")?;
        let default_export = words.next_if_eq(&"default").is_some();
        let _ = words.next_if_eq(&"abstract");
        words.next().filter(|word| *word == "class")?;
        let name = Self::identifier(words.next()?)?;
        let member = Self::member_name(&name, default_export);
        Some(Self { name, member })
    }

    fn member_name(name: &str, default_export: bool) -> String {
        if default_export {
            return String::from("default");
        }
        name.to_owned()
    }

    fn identifier(word: &str) -> Option<String> {
        let name: String = word
            .chars()
            .take_while(|character| Self::is_identifier_character(*character))
            .collect();
        Self::has_valid_start(&name).then_some(name)
    }

    fn is_identifier_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || character == '_' || character == '$'
    }

    fn has_valid_start(name: &str) -> bool {
        name.chars()
            .next()
            .is_some_and(|first| !first.is_ascii_digit())
    }
}
