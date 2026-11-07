pub(super) struct AnnotationMarks;

impl AnnotationMarks {
    const MARKS: [(&'static str, &'static [&'static str]); 5] = [
        ("rs", &["#["]),
        ("php", &["#["]),
        ("rb", &["#["]),
        ("cs", &["["]),
        ("go", &["// @", "#["]),
    ];
    const AT: &'static [&'static str] = &["@"];
    const NEWLINE: char = '\n';

    pub(super) fn precede(text: &str, start: usize, extension: &str) -> bool {
        let marks = Self::marks(extension);
        let line_start = Self::line_start(text, start);
        let head = text[line_start..start].trim_start();
        let previous = Self::previous_line(text, line_start).trim_start();
        Self::starts_with_mark(head, marks) | Self::starts_with_mark(previous, marks)
    }

    fn marks(extension: &str) -> &'static [&'static str] {
        Self::MARKS
            .iter()
            .find(|(language, _)| *language == extension)
            .map_or(Self::AT, |(_, marks)| marks)
    }

    fn starts_with_mark(line: &str, marks: &[&str]) -> bool {
        marks.iter().any(|mark| line.starts_with(mark))
    }

    fn line_start(text: &str, at: usize) -> usize {
        text[..at]
            .rfind(Self::NEWLINE)
            .map_or(0, |newline| newline + 1)
    }

    fn previous_line(text: &str, line_start: usize) -> &str {
        text[..line_start]
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
    }
}
