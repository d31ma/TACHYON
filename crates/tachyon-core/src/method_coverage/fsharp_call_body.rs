use super::{MethodHeader, SourceEdit};
use crate::fsharp_source::FsharpLines;

pub(crate) struct FsharpCallBody;

impl FsharpCallBody {
    const BODY_START: u8 = b'=';
    const LINE_BREAK: char = '\n';

    pub(crate) fn edits(header: &MethodHeader<'_>) -> Vec<SourceEdit> {
        Self::body_start(header)
            .and_then(|equals| Self::edit(header, equals + 1))
            .into_iter()
            .collect()
    }

    fn body_start(header: &MethodHeader<'_>) -> Option<usize> {
        let from = header.name_end()?;
        let rest = &header.masked[from..header.line_end()];
        MethodHeader::find_at_depth_zero(rest, Self::BODY_START).map(|found| from + found)
    }

    fn edit(header: &MethodHeader<'_>, after_equals: usize) -> Option<SourceEdit> {
        let rest = &header.masked[after_equals..header.line_end()];
        let expression = after_equals + rest.len() - rest.trim_start().len();
        if expression < header.line_end() {
            return Some(Self::split_line(header, expression));
        }
        let start = Self::next_body_line(header)?;
        Some(Self::before_line(header, start))
    }

    fn split_line(header: &MethodHeader<'_>, expression: usize) -> SourceEdit {
        let indent = " ".repeat(expression - header.start);
        let recorded = Self::recorded(header, &indent, Self::line_number(header, header.start));
        SourceEdit::insert(expression, format!("{}{recorded}{indent}", Self::LINE_BREAK))
    }

    fn before_line(header: &MethodHeader<'_>, start: usize) -> SourceEdit {
        let line = &header.masked[start..];
        let text = line.lines().next().unwrap_or_default();
        let indent = " ".repeat(text.len() - text.trim_start().len());
        SourceEdit::insert(start, Self::recorded(header, &indent, Self::line_number(header, start)))
    }

    fn recorded(header: &MethodHeader<'_>, indent: &str, line: usize) -> String {
        format!(
            "{indent}{}{}{}",
            header.hook,
            Self::LINE_BREAK,
            FsharpLines::directive(line, header.file)
        )
    }

    fn next_body_line(header: &MethodHeader<'_>) -> Option<usize> {
        let mut start = header.line_end() + 1;
        for line in header.masked.get(start..)?.split_inclusive(Self::LINE_BREAK) {
            if !line.trim().is_empty() {
                return Some(start);
            }
            start += line.len();
        }
        None
    }

    fn line_number(header: &MethodHeader<'_>, offset: usize) -> usize {
        header.masked[..offset].matches(Self::LINE_BREAK).count() + 1
    }
}
