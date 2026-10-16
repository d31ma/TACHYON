pub(super) struct CompilerErrorLine;

#[cfg(test)]
mod tests;

impl CompilerErrorLine {
    const GO: &'static str = "go";
    const GO_LOCATION: &'static str = ".go:";
    const SEVERITIES: [&'static str; 2] = ["error", "Error"];
    const PREAMBLES: [&'static str; 6] = ["warning", "Warning", "note", "Note", "help", "Help"];
    const SUMMARIES: [&'static str; 3] = [
        "too many errors",
        "aborting due to",
        "compilation terminated",
    ];

    pub(super) fn read<'a>(program: &str, detail: &'a str) -> &'a str {
        Self::first(program, detail)
            .or_else(|| Self::fallback(detail))
            .unwrap_or("no detail")
    }

    fn first<'a>(program: &str, detail: &'a str) -> Option<&'a str> {
        detail
            .lines()
            .map(str::trim)
            .find(|line| Self::substantive(program, line))
    }

    fn substantive(program: &str, line: &str) -> bool {
        [
            !Self::summary(line),
            !Self::preamble(line),
            Self::diagnostic(program, line),
        ] == [true; 3]
    }

    fn summary(line: &str) -> bool {
        let message = Self::go_message(line)
            .unwrap_or_else(|| line.split_once(": ").map_or(line, |(_, message)| message));
        Self::SUMMARIES
            .iter()
            .any(|summary| message.starts_with(summary))
    }

    fn preamble(line: &str) -> bool {
        if Self::go_location(line) {
            return Self::preamble_label(line);
        }
        line.split(": ")
            .take_while(|part| !Self::severity(part))
            .any(Self::preamble_label)
    }

    fn preamble_label(line: &str) -> bool {
        Self::PREAMBLES
            .iter()
            .filter_map(|label| line.strip_prefix(label))
            .any(Self::severity_suffix)
    }

    fn diagnostic(program: &str, line: &str) -> bool {
        if program == Self::GO {
            return Self::go_location(line);
        }
        line.split(": ").any(Self::severity)
    }

    fn go_location(line: &str) -> bool {
        Self::go_message(line).is_some()
    }

    fn go_message(line: &str) -> Option<&str> {
        line.split_once(Self::GO_LOCATION)
            .and_then(|(_, location)| Self::go_line_message(location))
    }

    fn go_line_message(location: &str) -> Option<&str> {
        location
            .split_once(':')
            .filter(|(number, _)| Self::line_number(number))
            .map(|(_, message)| Self::without_column(message.trim_start()))
    }

    fn without_column(message: &str) -> &str {
        message
            .split_once(':')
            .filter(|(number, _)| Self::line_number(number))
            .map_or(message, |(_, message)| message.trim_start())
    }

    fn line_number(number: &str) -> bool {
        [
            !number.is_empty(),
            number.bytes().all(|byte| byte.is_ascii_digit()),
        ] == [true; 2]
    }

    fn severity(line: &str) -> bool {
        let line = line.strip_prefix("fatal ").unwrap_or(line);
        Self::SEVERITIES
            .iter()
            .filter_map(|severity| line.strip_prefix(severity))
            .any(Self::severity_suffix)
    }

    fn severity_suffix(suffix: &str) -> bool {
        matches!(suffix.chars().next(), None | Some(':' | '[' | ' '))
    }

    fn fallback(detail: &str) -> Option<&str> {
        detail
            .lines()
            .map(str::trim)
            .find(|line| Self::content(line))
    }

    fn content(line: &str) -> bool {
        ![line.is_empty(), line.starts_with("# ")].contains(&true)
    }
}
