use std::collections::BTreeSet;

pub(super) struct SourceLines;

impl SourceLines {
    const NOTE_STARTS: [&'static str; 7] = ["@", "#", "//", "/*", "*", "--", "["];

    pub(super) fn with_offsets(text: &str) -> Vec<(usize, &str)> {
        let mut start = 0;
        text.split_inclusive('\n')
            .map(|line| {
                let at = start;
                start += line.len();
                (at, line)
            })
            .collect()
    }

    pub(super) fn indentation(line: &str) -> &str {
        let content = line.trim_start();
        &line[..line.len() - content.len()]
    }

    pub(super) fn is_blank(line: &str) -> bool {
        line.trim().is_empty()
    }

    pub(super) fn words(text: &str) -> BTreeSet<String> {
        text.split(|letter: char| !Self::is_word_letter(letter))
            .filter(|word| !word.is_empty())
            .map(str::to_owned)
            .collect()
    }

    pub(super) fn is_word_letter(letter: char) -> bool {
        [letter.is_ascii_alphanumeric(), letter == '_'].contains(&true)
    }

    pub(super) fn line_around(text: &str, at: usize) -> (usize, usize) {
        let before = text.get(..at).unwrap_or_default();
        let after = text.get(at..).unwrap_or_default();
        let start = before.rfind('\n').map_or(0, |offset| offset + 1);
        let end = after.find('\n').map_or(text.len(), |offset| at + offset);
        (start, end)
    }

    pub(super) fn is_note(line: &str) -> bool {
        let trimmed = line.trim_start();
        Self::NOTE_STARTS
            .iter()
            .any(|start| trimmed.starts_with(start))
    }
}
