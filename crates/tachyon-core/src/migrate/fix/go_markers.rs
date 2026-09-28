use super::edits::TextEdit;
use super::scan::SourceText;

pub(super) struct GoMarkerEdits;

impl GoMarkerEdits {
    pub(super) fn plan(text: &SourceText<'_>, start: usize, end: usize) -> Option<Vec<TextEdit>> {
        let source = text.source;
        let line_start = source[..start].rfind('\n').map_or(0, |at| at + 1);
        let own_line = source[line_start..start].trim().is_empty();
        let line_indent = &source[line_start..line_start + Self::spaces(source, line_start)];
        let indent = if own_line {
            line_indent.to_owned()
        } else {
            format!("{line_indent}\t")
        };
        let marker_line = format!("\n{indent}// @");
        let mut edits = if own_line {
            vec![TextEdit::replace(start, start + 2, "// @")]
        } else {
            vec![TextEdit::replace(
                Self::spaces_before(source, start),
                start + 2,
                &marker_line,
            )]
        };
        let mut depth = 0_usize;
        let mut at = start + 2;
        while at < end {
            let byte = text.bytes[at];
            if !text.is_code(at) {
                if byte == b'\n' || source[at..].starts_with("//") || source[at..].starts_with("/*")
                {
                    return None;
                }
                at += 1;
                continue;
            }
            if byte.is_ascii_whitespace() {
                let run_end = at + Self::spaces_and_lines(&source[at..end]);
                if source[at..run_end].contains('\n') {
                    let joined = Self::joins_tightly(text, at, run_end, end);
                    edits.push(TextEdit::replace(
                        at,
                        run_end,
                        if joined { "" } else { " " },
                    ));
                }
                at = run_end;
                continue;
            }
            match byte {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth = depth.saturating_sub(1),
                b',' if depth == 0 => {
                    let next = at + 1 + Self::spaces_and_lines(&source[at + 1..end]);
                    edits.push(TextEdit::replace(at, next, &marker_line));
                    at = next;
                    continue;
                }
                _ => {}
            }
            at += 1;
        }
        let line_end = source[end..]
            .find('\n')
            .map_or(source.len(), |offset| end + offset);
        let after = end + 1 + Self::spaces(source, end + 1);
        if source[end + 1..line_end].trim().is_empty() {
            edits.push(TextEdit::replace(end, end + 1, ""));
        } else {
            edits.push(TextEdit::replace(end, after, &format!("\n{indent}")));
        }
        Some(edits)
    }

    fn joins_tightly(text: &SourceText<'_>, from: usize, to: usize, end: usize) -> bool {
        let previous = text.source[..from].trim_end().bytes().last();
        let next = text.bytes.get(to).copied();
        matches!(previous, Some(b'(' | b'[' | b'{'))
            || to >= end
            || matches!(next, Some(b')' | b']' | b'}'))
    }

    fn spaces(source: &str, from: usize) -> usize {
        source[from..]
            .bytes()
            .take_while(|byte| matches!(byte, b' ' | b'\t'))
            .count()
    }

    fn spaces_and_lines(text: &str) -> usize {
        text.bytes().take_while(u8::is_ascii_whitespace).count()
    }

    fn spaces_before(source: &str, at: usize) -> usize {
        at - source[..at]
            .bytes()
            .rev()
            .take_while(|byte| matches!(byte, b' ' | b'\t'))
            .count()
    }
}
