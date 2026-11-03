use super::{MethodHeader, SourceEdit};

pub(crate) struct PythonBody;

impl PythonBody {
    const COLON: u8 = b':';
    const LINE_BREAK: char = '\n';

    pub(crate) fn edits(header: &MethodHeader<'_>) -> Vec<SourceEdit> {
        Self::colon(header)
            .map(|colon| Self::edit(header, colon))
            .into_iter()
            .collect()
    }

    fn colon(header: &MethodHeader<'_>) -> Option<usize> {
        let signature = &header.masked[header.start..];
        let found = MethodHeader::find_at_depth_zero(signature, Self::COLON)?;
        Some(header.start + found)
    }

    fn edit(header: &MethodHeader<'_>, colon: usize) -> SourceEdit {
        let line_end = Self::line_end(header.masked, colon);
        if header.masked[colon + 1..line_end].trim().is_empty() {
            return Self::new_line(header, line_end);
        }
        SourceEdit::insert(colon + 1, format!(" {};", header.hook))
    }

    fn line_end(text: &str, from: usize) -> usize {
        text[from..]
            .find(Self::LINE_BREAK)
            .map_or(text.len(), |found| from + found)
    }

    fn new_line(header: &MethodHeader<'_>, line_end: usize) -> SourceEdit {
        let next = (line_end + 1).min(header.source.len());
        let indentation = Self::indentation(&header.source[next..]);
        SourceEdit::insert(next, format!("{indentation}{}\n", header.hook))
    }

    fn indentation(text: &str) -> String {
        text.lines()
            .find(|line| !line.trim().is_empty())
            .map(|line| line[..line.len() - line.trim_start().len()].to_owned())
            .unwrap_or_default()
    }
}
