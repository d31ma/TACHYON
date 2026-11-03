#![allow(clippy::needless_bitwise_bool)]

use super::super::duplication::ImportWords;
use super::class_file::ClassFile;
use super::literal::Literal;

pub(super) struct LiteralReader;

impl LiteralReader {
    const QUOTES: [char; 2] = ['"', '\''];
    const RUST: &'static str = "rs";
    const PLAIN_NUMBERS: [&'static str; 6] = ["0", "1", "-1", "0.0", "1.0", "-1.0"];
    const MINUS: &'static str = "-";
    const POINT: &'static str = ".";
    const OPEN: &'static str = "(";
    const ANNOTATED: &'static str = "Annotated";
    const INTERPOLATIONS: [&'static str; 3] = ["$", "#{", "{{"];
    const CSHARP: &'static str = "cs";
    const PARAMETER_STARTS: [&'static str; 2] = ["(", ","];

    pub(super) fn read(class: &ClassFile) -> Vec<Literal> {
        Self::collect(class)
    }

    fn collect(class: &ClassFile) -> Vec<Literal> {
        let mut literals = Vec::new();
        let mut at = 0;
        while at < class.words.len() {
            let end = Self::skipped_end(class, at);
            literals.extend(Self::literal(class, at).filter(|_| end == at));
            at = end.max(at + 1);
        }
        literals
    }

    fn skipped_end(class: &ClassFile, at: usize) -> usize {
        Self::import_end(class, at)
            .or_else(|| class.tokens.marker_end(at))
            .or_else(|| Self::annotated_end(class, at))
            .or_else(|| Self::parameter_attribute_end(class, at))
            .unwrap_or(at)
    }

    fn annotated_end(class: &ClassFile, at: usize) -> Option<usize> {
        let annotated =
            (class.words.text(at) == Self::ANNOTATED) & (class.words.text(at + 1) == "[");
        annotated.then(|| class.words.after_group(at + 1))
    }

    fn parameter_attribute_end(class: &ClassFile, at: usize) -> Option<usize> {
        let csharp = class.extension == Self::CSHARP;
        let opens = class.words.text(at) == "[";
        let in_parameters = Self::PARAMETER_STARTS.contains(&class.words.text(at.wrapping_sub(1)));
        (csharp & opens & in_parameters).then(|| class.words.after_group(at))
    }

    fn import_end(class: &ClassFile, at: usize) -> Option<usize> {
        let first = class.words.line_start(at);
        let import = ImportWords::of(class.extension).contains(&class.words.text(at));
        let not_a_call = class.words.text(at + 1) != Self::OPEN;
        (first & import & not_a_call).then(|| class.index.line_end(at))
    }

    fn literal(class: &ClassFile, at: usize) -> Option<Literal> {
        let text = class.words.text(at);
        let string = Self::string_value(class, at, text);
        let number = Self::number_value(class, at, text);
        string
            .or(number)
            .map(|(value, number)| Literal { at, value, number })
    }

    fn string_value(class: &ClassFile, at: usize, text: &str) -> Option<(String, bool)> {
        let quoted = text.starts_with(Self::QUOTES) & (text.len() > 2);
        let closed = text.ends_with(&text[..1]);
        let rust_character = (class.extension == Self::RUST) & text.starts_with('\'');
        let interpolated = Self::INTERPOLATIONS.iter().any(|mark| text.contains(mark));
        let wanted = class.words.is_literal(at) & quoted & closed & !rust_character & !interpolated;
        wanted.then(|| (text[1..text.len() - 1].to_owned(), false))
    }

    fn number_value(class: &ClassFile, at: usize, text: &str) -> Option<(String, bool)> {
        let digit = text.starts_with(|first: char| first.is_ascii_digit());
        let fraction = (class.words.text(at.wrapping_sub(1)) == Self::POINT)
            & class.words.touches_previous(at);
        let value = Self::signed(class, at) + &Self::whole_number(class, at);
        let plain = Self::PLAIN_NUMBERS.contains(&value.as_str());
        (digit & !fraction & !plain).then_some((value, true))
    }

    fn signed(class: &ClassFile, at: usize) -> String {
        let minus = (class.words.text(at.wrapping_sub(1)) == Self::MINUS)
            & !class.words.value_before(at.wrapping_sub(1));
        [String::new(), Self::MINUS.to_owned()][usize::from(minus)].clone()
    }

    fn whole_number(class: &ClassFile, at: usize) -> String {
        let fraction = (class.words.text(at + 1) == Self::POINT)
            & class.words.touches_previous(at + 1)
            & class.words.touches_previous(at + 2)
            & class
                .words
                .text(at + 2)
                .starts_with(|first: char| first.is_ascii_digit());
        let digits = format!("{}.{}", class.words.text(at), class.words.text(at + 2));
        let whole = [class.words.text(at).to_owned(), digits][usize::from(fraction)].clone();
        Self::without_suffix(&whole.replace('_', ""))
    }

    fn without_suffix(number: &str) -> String {
        let prefixed =
            number.starts_with("0x") | number.starts_with("0b") | number.starts_with("0o");
        let digits: String = number
            .chars()
            .take_while(|character| character.is_ascii_digit() | (*character == '.'))
            .collect();
        [digits, number.to_owned()][usize::from(prefixed)].clone()
    }
}
