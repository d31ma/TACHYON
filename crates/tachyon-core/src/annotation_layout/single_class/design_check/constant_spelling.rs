#![allow(clippy::needless_bitwise_bool)]

use super::name_parts::NameParts;

pub(super) struct ConstantSpelling;

impl ConstantSpelling {
    const LONGEST_NAME: usize = 4;
    const NUMBER_NAME: &'static str = "limit";
    const TEXT_NAME: &'static str = "value";
    const TEMPLATES: [(&'static str, &'static str); 13] = [
        ("js", "static {UPPER} = {literal};"),
        ("ts", "static readonly {UPPER} = {literal};"),
        ("java", "private static final {type} {UPPER} = {literal};"),
        ("cs", "private const {type} {Pascal} = {literal};"),
        ("fs", "static let {camel} = {literal}"),
        ("kt", "companion object { const val {UPPER} = {literal} }"),
        ("swift", "static let {camel} = {literal}"),
        ("dart", "static const {camel} = {literal};"),
        ("php", "private const {UPPER} = {literal};"),
        ("py", "{UPPER} = {literal}"),
        ("rb", "{UPPER} = {literal}"),
        ("go", "func ({Class}) {camel}() {type} { return {literal} }"),
        ("rs", "const {UPPER}: {type} = {literal};"),
    ];
    const TYPES: [(&'static str, [&'static str; 3]); 4] = [
        ("java", ["String", "int", "double"]),
        ("cs", ["string", "int", "double"]),
        ("go", ["string", "int", "float64"]),
        ("rs", ["&str", "i64", "f64"]),
    ];

    pub(super) fn example(extension: &str, class: &str, literal: &str, number: bool) -> String {
        let template = Self::TEMPLATES
            .iter()
            .find(|(name, _)| *name == extension)
            .map_or("{UPPER} = {literal}", |(_, template)| *template);
        let words = Self::words(literal, number);
        template
            .replace("{UPPER}", &words.join("_").to_ascii_uppercase())
            .replace("{camel}", &Self::camel(&words))
            .replace("{Pascal}", &Self::pascal(&words))
            .replace("{type}", Self::type_name(extension, literal, number))
            .replace("{Class}", class)
            .replace("{literal}", literal)
    }

    fn words(literal: &str, number: bool) -> Vec<String> {
        let mut words: Vec<String> = NameParts::split(literal)
            .into_iter()
            .take(Self::LONGEST_NAME)
            .filter(|_| !number)
            .collect();
        let fallback = [Self::TEXT_NAME, Self::NUMBER_NAME][usize::from(number)];
        words.extend(words.is_empty().then(|| fallback.to_owned()));
        words
    }

    fn camel(words: &[String]) -> String {
        let pascal = Self::pascal(words);
        let first = words.first().map_or(0, String::len);
        format!(
            "{}{}",
            words.first().cloned().unwrap_or_default(),
            &pascal[first..]
        )
    }

    fn pascal(words: &[String]) -> String {
        words.iter().map(|word| Self::capitalized(word)).collect()
    }

    fn capitalized(word: &str) -> String {
        let mut letters = word.chars();
        letters
            .next()
            .map(|first| first.to_ascii_uppercase().to_string() + letters.as_str())
            .unwrap_or_default()
    }

    fn type_name(extension: &str, literal: &str, number: bool) -> &'static str {
        let kind = usize::from(number) + usize::from(number & literal.contains('.'));
        Self::TYPES
            .iter()
            .find(|(name, _)| *name == extension)
            .map_or("", |(_, types)| types[kind])
    }
}
