use super::scan::SourceText;
use crate::behavior::RoutingRename;

#[derive(Clone, Debug)]
pub(super) struct MarkerLine {
    pub(super) line_start: usize,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) name: String,
    pub(super) arguments: Option<(usize, usize)>,
}

pub(super) struct Marker;

impl Marker {
    const NAMESPACES: [&'static str; 2] = ["yon::", "tac::"];
    const PREFIXES: [(&'static str, &'static str); 5] = [
        ("rs", "#["),
        ("php", "#["),
        ("rb", "#["),
        ("go", "// @"),
        ("cs", "["),
    ];
    const UNBRACKETED_PREFIXES: [&'static str; 2] = ["@", "// @"];

    pub(super) fn prefix(extension: &str) -> &'static str {
        let known = Self::PREFIXES
            .iter()
            .find(|(written, _)| *written == extension);
        known.map_or("@", |(_, prefix)| *prefix)
    }

    pub(super) fn spelled(extension: &str, name: &str, arguments: &str) -> String {
        let written = Self::written_name(extension, name);
        let arguments = Self::argument_list(arguments);
        Self::wrapped(extension, &written, &arguments)
    }

    fn written_name(extension: &str, name: &str) -> String {
        if extension == "py" {
            return RoutingRename::snake_case(name);
        }
        String::from(name)
    }

    fn argument_list(arguments: &str) -> String {
        if arguments.is_empty() {
            return String::new();
        }
        format!("({arguments})")
    }

    fn wrapped(extension: &str, written: &str, arguments: &str) -> String {
        match Self::prefix(extension) {
            "@" => format!("@{written}{arguments}"),
            "// @" => format!("// @{written}{arguments}"),
            prefix => format!("{prefix}{written}{arguments}]"),
        }
    }

    pub(super) fn normalized(name: &str) -> String {
        name.replace('_', "").to_ascii_lowercase()
    }

    pub(super) fn quoted(text: &str) -> String {
        serde_json::Value::String(String::from(text)).to_string()
    }

    fn line_starts(text: &SourceText<'_>, from: usize) -> Vec<usize> {
        let mut starts = Vec::new();
        let mut line_start = from;
        while line_start < text.len() {
            starts.push(line_start);
            line_start = text.line_end(line_start);
        }
        starts
    }

    pub(super) fn lines(text: &SourceText<'_>, extension: &str, names: &[&str]) -> Vec<MarkerLine> {
        Self::line_starts(text, 0)
            .into_iter()
            .filter_map(|line_start| Self::at_line(text, extension, line_start))
            .filter(|marker| names.contains(&Self::normalized(&marker.name).as_str()))
            .collect()
    }

    pub(super) fn at_line(
        text: &SourceText<'_>,
        extension: &str,
        line_start: usize,
    ) -> Option<MarkerLine> {
        let start = line_start + text.indentation(line_start);
        let prefix = Self::prefix_at(text, extension, start);
        Self::starts_marker(text, start, prefix).then_some(())?;
        let name_start = Self::name_start(text, start + prefix.len());
        let name_end = text.word_end(name_start);
        (name_end > name_start).then_some(())?;
        let (arguments, after_arguments) = Self::arguments_at(text, name_end)?;
        let end = Self::marker_end(text, prefix, after_arguments)?;
        Some(MarkerLine {
            line_start,
            start,
            end,
            name: String::from(&text.source[name_start..name_end]),
            arguments,
        })
    }

    fn prefix_at(text: &SourceText<'_>, extension: &str, start: usize) -> &'static str {
        let rust_style = text.source[start..].starts_with("#[");
        if [extension == "go", rust_style] == [true, true] {
            return "#[";
        }
        Self::prefix(extension)
    }

    fn starts_marker(text: &SourceText<'_>, start: usize, prefix: &str) -> bool {
        let code_start = start + usize::from(prefix == "// @") * 3;
        let written = text.source[start..].starts_with(prefix);
        [text.is_code(code_start), written] == [true, true]
    }

    fn name_start(text: &SourceText<'_>, after_prefix: usize) -> usize {
        let mut name_start = after_prefix;
        for namespace in Self::NAMESPACES {
            let written = text.source[name_start..].starts_with(namespace);
            name_start += usize::from(written) * namespace.len();
        }
        name_start
    }

    fn arguments_at(
        text: &SourceText<'_>,
        name_end: usize,
    ) -> Option<(Option<(usize, usize)>, usize)> {
        if text.bytes.get(name_end) != Some(&b'(') {
            return Some((None, name_end));
        }
        let close = text.matching(name_end)?;
        Some((Some((name_end + 1, close)), close + 1))
    }

    fn marker_end(text: &SourceText<'_>, prefix: &str, after_arguments: usize) -> Option<usize> {
        if Self::UNBRACKETED_PREFIXES.contains(&prefix) {
            return Some(after_arguments);
        }
        let closed = text.bytes.get(after_arguments) == Some(&b']');
        closed.then_some(after_arguments + 1)
    }

    pub(super) fn next_declaration(
        text: &SourceText<'_>,
        extension: &str,
        from: usize,
    ) -> Option<usize> {
        Self::line_starts(text, text.line_end(from))
            .into_iter()
            .find_map(|line_start| Self::declaration_at(text, extension, line_start))
    }

    fn declaration_at(text: &SourceText<'_>, extension: &str, line_start: usize) -> Option<usize> {
        let is_marker = Self::at_line(text, extension, line_start).is_some();
        let is_blank = text.is_blank_line(line_start);
        ([is_marker, is_blank] == [false, false]).then_some(())?;
        let at = line_start + text.indentation(line_start);
        text.is_code(at).then_some(at)
    }

    pub(super) fn indentation<'a>(text: &SourceText<'a>, line_start: usize) -> &'a str {
        &text.source[line_start..line_start + text.indentation(line_start)]
    }
}
