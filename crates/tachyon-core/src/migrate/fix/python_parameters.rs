use super::edits::TextEdit;
use super::scan::SourceText;

pub(super) struct PythonParameterEdits;

struct DecoratorRun {
    start: usize,
    indent: String,
    markers: Vec<String>,
    only_markers: bool,
}

impl PythonParameterEdits {
    const IMPORT: &'static str = "from typing import Annotated\n";

    fn is_marker(name: &str) -> bool {
        crate::annotation_rules::AnnotationRules::find(name)
            .is_some_and(crate::annotation_rules::Rule::python_annotated)
    }

    pub(super) fn annotated_declaration(declaration: &str, markers: &[String]) -> String {
        let (declared, default) = declaration
            .split_once('=')
            .map_or((declaration, None), |(declared, value)| {
                (declared, Some(value.trim()))
            });
        let (name, kind) = declared
            .split_once(':')
            .map_or((declared.trim(), "object"), |(name, kind)| {
                (name.trim(), kind.trim())
            });
        let markers: Vec<&str> = markers
            .iter()
            .map(|marker| marker.trim_start_matches('@'))
            .collect();
        let default = default
            .map(|value| format!(" = {value}"))
            .unwrap_or_default();
        format!("{name}: Annotated[{kind}, {}]{default}", markers.join(", "))
    }

    pub(super) fn plan_fields(text: &SourceText<'_>) -> Vec<(usize, TextEdit)> {
        let mut edits = Vec::new();
        let mut run: Option<DecoratorRun> = None;
        let mut start = 0;
        while start < text.len() {
            let end = Self::line_end(text, start);
            let line = &text.source[start..end];
            let first = start + line.len() - line.trim_start().len();
            if first == end || !text.is_code(first) {
                start = end + 1;
                continue;
            }
            if text.bytes[first] == b'@' {
                let name_end = text.word_end(first + 1);
                let marker_end = Self::marker_end(text, name_end).unwrap_or(end);
                let line_end = Self::line_end(text, marker_end);
                let current = run.get_or_insert_with(|| DecoratorRun {
                    start,
                    indent: text.source[start..first].to_owned(),
                    markers: Vec::new(),
                    only_markers: true,
                });
                let whole_line = text.source[marker_end..line_end].trim().is_empty();
                if Self::is_marker(&text.source[first + 1..name_end]) && whole_line {
                    current
                        .markers
                        .push(text.source[first + 1..marker_end].to_owned());
                } else {
                    current.only_markers = false;
                }
                start = line_end + 1;
                continue;
            }
            if let Some(found) = run.take()
                && found.only_markers
                && let Some(edit) = Self::annotate_field(text, &found, first, end)
            {
                edits.push((found.start, edit));
            }
            start = end + 1;
        }
        edits
    }

    fn line_end(text: &SourceText<'_>, start: usize) -> usize {
        text.source[start..]
            .find('\n')
            .map_or(text.len(), |offset| start + offset)
    }

    fn annotate_field(
        text: &SourceText<'_>,
        run: &DecoratorRun,
        first: usize,
        end: usize,
    ) -> Option<TextEdit> {
        let name_end = text.word_end(first);
        let name = &text.source[first..name_end];
        let (kind, rest) = Self::field_kind(text, name_end, end);
        let tail = rest.trim_start();
        let plain_tail = [tail.is_empty(), tail.starts_with(['=', '#'])].contains(&true);
        let rejected = [
            name.is_empty(),
            kind.is_empty(),
            kind.starts_with("Annotated["),
            !plain_tail,
        ];
        (!rejected.contains(&true)).then_some(())?;
        let replacement = format!(
            "{}{name}: Annotated[{kind}, {}]{rest}",
            run.indent,
            run.markers.join(", ")
        );
        Some(TextEdit::replace(run.start, end, &replacement))
    }

    fn field_kind<'a>(text: &SourceText<'a>, name_end: usize, end: usize) -> (&'a str, &'a str) {
        let after = Self::skip_space(text, name_end);
        if text.bytes.get(after) != Some(&b':') {
            return ("object", &text.source[name_end..end]);
        }
        let kind_end = Self::kind_end(text, after + 1, end);
        let written = &text.source[after + 1..kind_end];
        let kept = after + 1 + written.trim_end().len();
        (written.trim(), &text.source[kept..end])
    }

    fn kind_end(text: &SourceText<'_>, start: usize, end: usize) -> usize {
        let mut depth = 0_usize;
        for at in start..end {
            if !text.is_code(at) {
                if depth == 0 {
                    return at;
                }
                continue;
            }
            match text.bytes[at] {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth = depth.saturating_sub(1),
                b'=' if depth == 0 => return at,
                _ => {}
            }
        }
        end
    }

    pub(super) fn plan(text: &SourceText<'_>) -> Vec<(usize, TextEdit)> {
        text.source
            .match_indices('@')
            .filter_map(|(at, _)| Self::parameter_edit(text, at).map(|edit| (at, edit)))
            .collect()
    }

    fn parameter_edit(text: &SourceText<'_>, at: usize) -> Option<TextEdit> {
        let parameter_marker = [text.is_code(at), Self::opens_parameter(text, at)];
        (parameter_marker == [true, true]).then_some(())?;
        let name_end = text.word_end(at + 1);
        Self::is_marker(&text.source[at + 1..name_end]).then_some(())?;
        let marker_end = Self::marker_end(text, name_end)?;
        let marker = &text.source[at + 1..marker_end];
        let start = Self::skip_space(text, marker_end);
        Self::wrap(text, at, start, marker)
    }

    pub(super) fn wrap(
        text: &SourceText<'_>,
        replace_from: usize,
        start: usize,
        marker: &str,
    ) -> Option<TextEdit> {
        let end = Self::parameter_end(text, start)?;
        let parameter = text.source[start..end].trim_end();
        let rejected = [parameter.contains('='), parameter.starts_with(['*', '@'])];
        (rejected == [false, false]).then_some(())?;
        let (name, kind) = parameter
            .split_once(':')
            .map_or((parameter, "object"), |(name, kind)| {
                (name.trim(), kind.trim())
            });
        let replacement = format!("{name}: Annotated[{kind}, {marker}]");
        Some(TextEdit::replace(
            replace_from,
            start + parameter.len(),
            &replacement,
        ))
    }

    pub(super) fn import(source: &str) -> Option<TextEdit> {
        let imported = source
            .lines()
            .any(|line| line.trim() == "from typing import Annotated");
        (!imported).then_some(())?;
        let after_future_imports = source
            .split_inclusive('\n')
            .take_while(|line| line.starts_with("from __future__ import "))
            .map(str::len)
            .sum();
        Some(TextEdit::insert(after_future_imports, Self::IMPORT))
    }

    fn opens_parameter(text: &SourceText<'_>, at: usize) -> bool {
        let before = text.source[..at].trim_end();
        before.ends_with('(') || before.ends_with(',')
    }

    fn marker_end(text: &SourceText<'_>, name_end: usize) -> Option<usize> {
        if text.bytes.get(name_end) != Some(&b'(') {
            return Some(name_end);
        }
        text.matching(name_end).map(|close| close + 1)
    }

    fn skip_space(text: &SourceText<'_>, mut at: usize) -> usize {
        while text.bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        at
    }

    fn parameter_end(text: &SourceText<'_>, start: usize) -> Option<usize> {
        let mut depth = 0_usize;
        for at in start..text.len() {
            if !text.is_code(at) {
                continue;
            }
            match text.bytes[at] {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' | b',' if depth == 0 => return Some(at),
                b')' | b']' | b'}' => depth -= 1,
                _ => {}
            }
        }
        None
    }
}
