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
        let rest = header.masked[at..header.line_end().max(at)].trim_start();
        if rest.starts_with(Self::ENDLESS) {
            return Self::endless(header, at);
        }
        vec![SourceEdit::insert(
            Self::written_end(header),
            format!("; {}", header.hook),
        )]
    }

    fn endless(header: &MethodHeader<'_>, at: usize) -> Vec<SourceEdit> {
        let equals = header.first_after(at, &[Self::ENDLESS]).unwrap_or(at);
        vec![
            SourceEdit::insert(equals + 1, format!(" ({};", header.hook)),
            SourceEdit::insert(Self::written_end(header), String::from(")")),
        ]
    }

    fn written_end(header: &MethodHeader<'_>) -> usize {
        header.start + header.line().trim_end().len()
    }
}
