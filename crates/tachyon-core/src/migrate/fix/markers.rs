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

    pub(super) fn prefix(extension: &str) -> &'static str {
        match extension {
            "rs" | "php" | "rb" => "#[",
            "go" => "// @",
            "cs" => "[",
            _ => "@",
        }
    }

    pub(super) fn spelled(extension: &str, name: &str, arguments: &str) -> String {
        let written = if extension == "py" {
            RoutingRename::snake_case(name)
        } else {
            String::from(name)
        };
        let arguments = if arguments.is_empty() {
            String::new()
        } else {
            format!("({arguments})")
        };
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

    pub(super) fn lines(text: &SourceText<'_>, extension: &str, names: &[&str]) -> Vec<MarkerLine> {
        let mut found = Vec::new();
        let mut line_start = 0;
        while line_start < text.len() {
            if let Some(marker) = Self::at_line(text, extension, line_start)
                && names.contains(&Self::normalized(&marker.name).as_str())
            {
                found.push(marker);
            }
            line_start = text.line_end(line_start);
        }
        found
    }

    pub(super) fn at_line(
        text: &SourceText<'_>,
        extension: &str,
        line_start: usize,
    ) -> Option<MarkerLine> {
        let line = text.line_text(line_start);
        let start = line_start + line.len() - line.trim_start().len();
        let prefix = if extension == "go" && text.source[start..].starts_with("#[") {
            "#["
        } else {
            Self::prefix(extension)
        };
        let code_start = start + usize::from(prefix == "// @") * 3;
        if !text.is_code(code_start) || !text.source[start..].starts_with(prefix) {
            return None;
        }
        let mut name_start = start + prefix.len();
        for namespace in Self::NAMESPACES {
            if text.source[name_start..].starts_with(namespace) {
                name_start += namespace.len();
            }
        }
        let name_end = text.word_end(name_start);
        if name_end == name_start {
            return None;
        }
        let mut end = name_end;
        let mut arguments = None;
        if text.bytes.get(end) == Some(&b'(') {
            let close = text.matching(end)?;
            arguments = Some((end + 1, close));
            end = close + 1;
        }
        if !matches!(prefix, "@" | "// @") {
            if text.bytes.get(end) != Some(&b']') {
                return None;
            }
            end += 1;
        }
        Some(MarkerLine {
            line_start,
            start,
            end,
            name: String::from(&text.source[name_start..name_end]),
            arguments,
        })
    }

    pub(super) fn next_declaration(
        text: &SourceText<'_>,
        extension: &str,
        from: usize,
    ) -> Option<usize> {
        let mut line_start = text.line_end(from);
        while line_start < text.len() {
            let is_marker = Self::at_line(text, extension, line_start).is_some();
            if !is_marker && !text.is_blank_line(line_start) {
                let line = text.line_text(line_start);
                let at = line_start + line.len() - line.trim_start().len();
                if text.is_code(at) {
                    return Some(at);
                }
            }
            line_start = text.line_end(line_start);
        }
        None
    }

    pub(super) fn indentation<'a>(text: &SourceText<'a>, line_start: usize) -> &'a str {
        &text.source[line_start..line_start + text.indentation(line_start)]
    }
}
