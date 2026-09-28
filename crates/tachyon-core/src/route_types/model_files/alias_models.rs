use super::super::{Model, Shape, TypeText};

pub(super) struct AliasModels;

impl AliasModels {
    const VISIBILITY: [&'static str; 6] = [
        "pub(crate) ",
        "pub ",
        "public ",
        "internal ",
        "private ",
        "fileprivate ",
    ];

    pub(super) fn read(extension: &str, code: &str) -> Vec<(String, Model)> {
        Self::keyword(extension)
            .map(|keyword| Self::aliases(extension, keyword, code))
            .unwrap_or_default()
    }

    fn aliases(extension: &str, keyword: &str, code: &str) -> Vec<(String, Model)> {
        code.lines()
            .filter(|line| !line.starts_with(char::is_whitespace))
            .filter_map(|line| Self::alias(extension, keyword, line))
            .collect()
    }

    fn keyword(extension: &str) -> Option<&'static str> {
        match extension {
            "rs" => Some("type "),
            "kt" | "swift" => Some("typealias "),
            "dart" => Some("typedef "),
            _ => None,
        }
    }

    fn alias(extension: &str, keyword: &str, line: &str) -> Option<(String, Model)> {
        let rest = Self::without_visibility(line.trim()).strip_prefix(keyword)?;
        let (name, text) = rest.split_once('=')?;
        let name = Self::name(name)?;
        let text = text.trim().trim_end_matches(';').trim();
        let shape = TypeText::shape(extension, text).unwrap_or(Shape::Any);
        Some((name.to_owned(), Model::Alias(shape)))
    }

    fn without_visibility(line: &str) -> &str {
        Self::VISIBILITY
            .iter()
            .find_map(|word| line.strip_prefix(word))
            .unwrap_or(line)
    }

    fn name(text: &str) -> Option<&str> {
        let name = text.split('<').next().unwrap_or(text).trim();
        let plain = name
            .bytes()
            .all(|byte| [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true));
        let starts_as_name = name
            .bytes()
            .next()
            .is_some_and(|byte| !byte.is_ascii_digit());
        (![plain, starts_as_name].contains(&false)).then_some(name)
    }
}
