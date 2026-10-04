use super::{Field, Input};
use crate::Failure;
use std::ops::Range;

pub(super) struct PythonFields;

impl PythonFields {
    const OPENERS: &'static [u8] = b"([{";
    const CLOSERS: &'static [u8] = b")]}";
    const HIDDEN_STARTS: [char; 2] = ['@', '#'];

    pub(super) fn read(input: &Input<'_>, body: Range<usize>) -> Result<Vec<Field>, Failure> {
        let direct_indent = super::minimum_indentation(&input.code[body.clone()]);
        Self::fields(input, body, direct_indent)
    }

    fn fields(
        input: &Input<'_>,
        body: Range<usize>,
        direct_indent: usize,
    ) -> Result<Vec<Field>, Failure> {
        let mut fields = Vec::new();
        let mut open_brackets = 0_i64;
        let mut offset = body.start;
        for segment in input.code[body].split_inclusive('\n') {
            let outside_brackets = open_brackets == 0;
            open_brackets += Self::bracket_change(segment);
            let span = Self::span(input, offset, segment);
            offset += segment.len();
            let direct = [outside_brackets, Self::direct(segment, direct_indent)];
            fields.extend(Self::field_when(direct == [true, true], input, span)?);
        }
        Ok(fields)
    }

    fn direct(segment: &str, direct_indent: usize) -> bool {
        let text = segment.trim();
        let indent = segment.len() - segment.trim_start().len();
        [
            !text.is_empty(),
            !text.starts_with(Self::HIDDEN_STARTS),
            indent == direct_indent,
        ] == [true, true, true]
    }

    fn span(input: &Input<'_>, offset: usize, segment: &str) -> Range<usize> {
        let start = offset + segment.len() - segment.trim_start().len();
        let code_end = start + segment.trim().len();
        let line_end = offset + segment.len();
        start..code_end + super::trailing_text_length(&input.source[code_end..line_end])
    }

    fn field_when(
        direct: bool,
        input: &Input<'_>,
        span: Range<usize>,
    ) -> Result<Option<Field>, Failure> {
        direct
            .then(|| Self::field(&input.code[span.clone()], &input.source[span.clone()]))
            .transpose()
            .map(Option::flatten)
            .map_err(|message| input.failure(span.start, message))
    }

    fn field(code: &str, source: &str) -> Result<Option<Field>, String> {
        Self::declared_name(code)
            .map(|(name, colon)| Self::typed_field(name, &code[colon + 1..], &source[colon + 1..]))
            .transpose()
            .map(Option::flatten)
    }

    fn declared_name(code: &str) -> Option<(&str, usize)> {
        let colon = code.find(':')?;
        let name = code[..colon].trim();
        super::super::identifier(name).then_some((name, colon))
    }

    fn typed_field(name: &str, code: &str, source: &str) -> Result<Option<Field>, String> {
        Self::require_one_statement(name, code)?;
        let assignment = Self::top_level(code, b'=');
        let annotation = &source[..assignment.unwrap_or(code.len())];
        let type_source = super::without_field_modifiers(annotation.trim().trim_end_matches(';'));
        let field = Field {
            name: name.to_owned(),
            is_static: type_source.starts_with("ClassVar"),
            type_source,
            uninitialized: assignment.is_none(),
            initializer: assignment.map(|at| Self::initializer(&source[at + 1..])),
        };
        Ok((!field.type_source.is_empty()).then_some(field))
    }

    fn initializer(text: &str) -> String {
        text.trim().trim_end_matches(';').trim_end().to_owned()
    }

    fn require_one_statement(name: &str, code: &str) -> Result<(), String> {
        Self::top_level(code, b';')
            .map_or("", |separator| code[separator + 1..].trim())
            .is_empty()
            .then_some(())
            .ok_or_else(|| {
                format!(
                    "The Python field '{name}' shares its line with another statement; put each field on its own line"
                )
            })
    }

    fn top_level(code: &str, wanted: u8) -> Option<usize> {
        code.bytes()
            .scan(0_i64, |depth, byte| {
                *depth += Self::depth_change(byte);
                Some((*depth, byte))
            })
            .position(|(depth, byte)| [depth == 0, byte == wanted] == [true, true])
    }

    fn bracket_change(code: &str) -> i64 {
        code.bytes().map(Self::depth_change).sum()
    }

    fn depth_change(byte: u8) -> i64 {
        i64::from(Self::OPENERS.contains(&byte)) - i64::from(Self::CLOSERS.contains(&byte))
    }
}
