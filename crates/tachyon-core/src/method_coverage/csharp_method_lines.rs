use std::borrow::Cow;

pub(super) struct CsharpMethodLines;

impl CsharpMethodLines {
    const LANGUAGE: &'static str = "cs";
    const PUBLIC: &'static str = "public ";
    const HIDDEN: [&'static str; 4] = ["private", "protected", "internal", "static"];
    const MIN_SIGNATURE_WORDS: usize = 3;

    pub(super) fn prepare<'a>(language: &str, source: &'a str) -> Cow<'a, str> {
        Self::selected(language, source)
    }

    pub(super) fn hidden(line: &str) -> bool {
        Self::signature(line)
            .split_whitespace()
            .any(|word| Self::HIDDEN.contains(&word))
    }

    fn signature(line: &str) -> &str {
        line.split('(').next().unwrap_or_default()
    }

    fn selected<'a>(language: &str, source: &'a str) -> Cow<'a, str> {
        if language == Self::LANGUAGE {
            return Cow::Owned(Self::joined_headers(source));
        }
        Cow::Borrowed(source)
    }

    fn joined_headers(source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        lines
            .iter()
            .enumerate()
            .map(|(index, line)| Self::with_opening(line, lines.get(index + 1).copied()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn with_opening(line: &str, next: Option<&str>) -> String {
        if Self::owns_next_brace(line, next) {
            return format!("{line} {{");
        }
        line.to_owned()
    }

    fn owns_next_brace(line: &str, next: Option<&str>) -> bool {
        (
            line.trim_start().starts_with(Self::PUBLIC),
            line.trim_end().ends_with(')'),
            next.is_some_and(|text| text.trim_start().starts_with('{')),
            line.contains('='),
            Self::signature(line).split_whitespace().count() >= Self::MIN_SIGNATURE_WORDS,
        ) == (true, true, true, false, true)
    }
}
