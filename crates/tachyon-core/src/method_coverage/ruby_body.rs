use super::{MethodHeader, SourceEdit};

pub(crate) struct RubyBody;

impl RubyBody {
    const PARAMETERS: char = '(';
    const ENDLESS: char = '=';

    pub(crate) fn edits(header: &MethodHeader<'_>) -> Vec<SourceEdit> {
        header
            .name_end()
            .map(|name_end| Self::edits_after(header, Self::signature_end(header, name_end)))
            .unwrap_or_default()
    }

    fn signature_end(header: &MethodHeader<'_>, name_end: usize) -> usize {
        if header.masked[name_end..].starts_with(Self::PARAMETERS) {
            return MethodHeader::closing(header.masked, name_end).unwrap_or(name_end);
        }
        name_end
    }

    fn edits_after(header: &MethodHeader<'_>, at: usize) -> Vec<SourceEdit> {
        let signature_line = Self::signature_line(header, at);
        let line_end = signature_line + Self::line_at(header, signature_line).len();
        let rest = header.masked[at..line_end.max(at)].trim_start();
        if rest.starts_with(Self::ENDLESS) {
            return Self::endless(header, at, signature_line);
        }
        vec![SourceEdit::insert(
            Self::written_end(header, signature_line),
            format!("; {}", header.hook),
        )]
    }

    fn endless(header: &MethodHeader<'_>, at: usize, signature_line: usize) -> Vec<SourceEdit> {
        let equals = header.first_after(at, &[Self::ENDLESS]).unwrap_or(at);
        vec![
            SourceEdit::insert(equals + 1, format!(" ({};", header.hook)),
            SourceEdit::insert(Self::written_end(header, signature_line), String::from(")")),
        ]
    }

    fn signature_line(header: &MethodHeader<'_>, at: usize) -> usize {
        header.masked[..at]
            .rfind('\n')
            .map_or(header.start, |newline| newline + 1)
            .max(header.start)
    }

    fn line_at<'a>(header: &MethodHeader<'a>, start: usize) -> &'a str {
        header.masked[start..].lines().next().unwrap_or_default()
    }

    fn written_end(header: &MethodHeader<'_>, signature_line: usize) -> usize {
        signature_line + Self::line_at(header, signature_line).trim_end().len()
    }
}
