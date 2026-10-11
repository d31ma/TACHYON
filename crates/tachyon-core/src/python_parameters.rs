use crate::ascii_name::AsciiName;
use std::ops::Range;

pub(crate) struct PythonAnnotated {
    pub(crate) name: String,
    pub(crate) type_text: String,
    pub(crate) marker: String,
    pub(crate) argument: Option<String>,
    pub(crate) erased: [Range<usize>; 2],
}

pub(crate) struct PythonMarker {
    pub(crate) written: String,
    pub(crate) arguments: String,
    pub(crate) at: usize,
    pub(crate) erased: Range<usize>,
    pub(crate) declaration: usize,
    pub(crate) parameters: Option<usize>,
}

pub(crate) struct PythonMarkers {
    pub(crate) markers: Vec<PythonMarker>,
    pub(crate) erased: Vec<Range<usize>>,
}

impl PythonMarkers {
    pub(crate) fn blanked(&self, text: &str) -> String {
        let mut bytes = text.as_bytes().to_vec();
        for range in &self.erased {
            for byte in &mut bytes[range.clone()] {
                if !matches!(*byte, b'\n' | b'\r') {
                    *byte = b' ';
                }
            }
        }
        String::from_utf8(bytes).unwrap_or_else(|_| text.to_owned())
    }
}

impl PythonAnnotated {
    const OPENING: &'static str = "Annotated[";

    pub(crate) fn markers(
        code: &str,
        source: &str,
        is_marker: impl Fn(&str) -> bool,
    ) -> PythonMarkers {
        let mut found = PythonMarkers {
            markers: Vec::new(),
            erased: Vec::new(),
        };
        for (at, _) in code.match_indices(Self::OPENING) {
            let starts_word = code[..at]
                .bytes()
                .next_back()
                .is_none_or(|byte| !(byte.is_ascii_alphanumeric() || byte == b'_'));
            let Some((declaration, parameters)) = Self::declaration(code, at) else {
                continue;
            };
            let inner = at + Self::OPENING.len();
            let Some(close) = Self::closing(code, inner - 1) else {
                continue;
            };
            if !starts_word {
                continue;
            }
            let items = Self::items(code, inner, close);
            let mut only_markers = items.len() > 1;
            for pair in items.windows(2) {
                let Some(marker) = Self::marker(code, source, &pair[0], &pair[1], &is_marker)
                else {
                    only_markers = false;
                    continue;
                };
                found.erased.push(marker.erased.clone());
                found.markers.push(PythonMarker {
                    declaration,
                    parameters,
                    ..marker
                });
            }
            if only_markers {
                found.erased.push(at..inner);
                found.erased.push(close..close + 1);
            }
        }
        found
    }

    fn marker(
        code: &str,
        source: &str,
        previous: &Range<usize>,
        item: &Range<usize>,
        is_marker: &impl Fn(&str) -> bool,
    ) -> Option<PythonMarker> {
        let written = &code[item.clone()];
        let text = written.trim();
        let start = item.start + written.len() - written.trim_start().len();
        let length = Self::name_length(text);
        let name = &text[..length];
        let rest = text[length..].trim_start();
        if !is_marker(name) {
            return None;
        }
        let arguments = if rest.is_empty() {
            String::new()
        } else if rest.starts_with('(') && rest.ends_with(')') {
            let open = start + text.len() - rest.len();
            source
                .get(open + 1..start + text.len() - 1)?
                .trim()
                .to_owned()
        } else {
            return None;
        };
        Some(PythonMarker {
            written: name.to_owned(),
            arguments,
            at: start,
            erased: previous.end..item.end,
            declaration: 0,
            parameters: None,
        })
    }

    fn name_length(text: &str) -> usize {
        AsciiName::leading(text).len()
    }

    fn declaration(code: &str, at: usize) -> Option<(usize, Option<usize>)> {
        let before = code[..at].trim_end();
        let before = before.strip_suffix(':')?.trim_end();
        let length = before
            .bytes()
            .rev()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        if length == 0 {
            return None;
        }
        let name = before.len() - length;
        let parameters = Self::open_parenthesis(code, name);
        if parameters.is_some() {
            let previous = code[..name].trim_end();
            return (previous.ends_with('(') || previous.ends_with(','))
                .then_some((name, parameters));
        }
        let line = &code[code[..name].rfind('\n').map_or(0, |end| end + 1)..name];
        line.trim().is_empty().then_some((name, None))
    }

    fn open_parenthesis(code: &str, at: usize) -> Option<usize> {
        let mut depth = 0_usize;
        for (position, byte) in code[..at].bytes().enumerate().rev() {
            match byte {
                b')' | b']' | b'}' => depth += 1,
                b'(' if depth == 0 => return Some(position),
                b'(' | b'[' | b'{' => depth = depth.checked_sub(1)?,
                _ => {}
            }
        }
        None
    }

    fn closing(code: &str, open: usize) -> Option<usize> {
        let mut depth = 0_usize;
        for (offset, byte) in code.as_bytes()[open..].iter().enumerate() {
            match byte {
                b'[' | b'(' | b'{' => depth += 1,
                b']' | b')' | b'}' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(open + offset);
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub(crate) fn read(
        code: &str,
        source: &str,
        at: usize,
        is_marker: impl Fn(&str) -> bool,
    ) -> Option<Self> {
        let colon = code.find(':')?;
        let name = code[..colon].trim();
        let plain_name = !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
        if !plain_name {
            return None;
        }
        let after = &code[colon + 1..];
        let kind_start = colon + 1 + after.len() - after.trim_start().len();
        let kind_end = code.trim_end().len();
        let kind = code.get(kind_start..kind_end)?;
        if !kind.starts_with(Self::OPENING) || !kind.ends_with(']') {
            return None;
        }
        let inner_start = kind_start + Self::OPENING.len();
        let items = Self::items(code, inner_start, kind_end - 1);
        let first = items.first()?.clone();
        for item in items.iter().skip(1) {
            let written = &code[item.clone()];
            let text = written.trim();
            let start = item.start + written.len() - written.trim_start().len();
            let length = text
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
            let marker = &text[..length];
            if !is_marker(marker) {
                continue;
            }
            let rest = text[length..].trim();
            let argument = if rest.is_empty() {
                None
            } else if rest.starts_with('(') && rest.ends_with(')') {
                let open = start + text.len() - rest.len();
                let close = start + text.len() - 1;
                Some(source.get(open + 1..close)?.trim().to_owned())
            } else {
                return None;
            };
            return Some(Self {
                name: name.to_owned(),
                type_text: source.get(first.clone())?.trim().to_owned(),
                marker: marker.to_owned(),
                argument,
                erased: [
                    at + kind_start..at + inner_start,
                    at + first.end..at + kind_end,
                ],
            });
        }
        None
    }

    pub(crate) fn items(code: &str, start: usize, end: usize) -> Vec<Range<usize>> {
        let mut items = Vec::new();
        let mut depth = 0_i32;
        let mut item_start = start;
        for (offset, byte) in code.as_bytes()[start..end].iter().enumerate() {
            match byte {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b',' if depth == 0 => {
                    items.push(item_start..start + offset);
                    item_start = start + offset + 1;
                }
                _ => {}
            }
        }
        items.push(item_start..end);
        items
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::PythonAnnotated;

    #[test]
    fn marker_names_are_the_leading_ascii_word_characters() {
        let alphabet = (0..=0x7f_u8)
            .map(char::from)
            .chain(['é', 'Ü', 'ß', '٣'])
            .collect::<Vec<_>>();
        let mut texts = vec![String::new()];
        for first in &alphabet {
            texts.push(first.to_string());
            texts.extend(alphabet.iter().map(|second| format!("{first}{second}")));
        }
        for text in texts {
            let length = text
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
            assert_eq!(PythonAnnotated::name_length(&text), length, "{text:?}");
        }
    }

    #[test]
    fn an_annotated_parameter_names_its_type_marker_and_argument() {
        let source = "view: Annotated[list[str], from_query(\"view\")]";
        let code = "view: Annotated[list[str], from_query(      )]";
        let read = PythonAnnotated::read(code, source, 10, |name| name == "from_query")
            .expect("annotated parameter");
        assert_eq!(read.name, "view");
        assert_eq!(read.type_text, "list[str]");
        assert_eq!(read.marker, "from_query");
        assert_eq!(read.argument.as_deref(), Some("\"view\""));
        assert_eq!(
            &source[read.erased[0].start - 10..read.erased[0].end - 10],
            "Annotated["
        );
        assert_eq!(
            &source[read.erased[1].start - 10..read.erased[1].end - 10],
            ", from_query(\"view\")]"
        );
    }

    #[test]
    fn fields_and_parameters_hold_their_markers_inside_annotated() {
        let source = "class Order:\n    title: Annotated[str, required, length(1, 80)] = ''\n    note: Annotated[str, Doc(\"x\"), not_empty]\n    def find(self, key: Annotated[str, cache_key]):\n        size: Annotated[int, min(1)] = 1\n";
        let found = PythonAnnotated::markers(source, source, |name| {
            matches!(
                name,
                "required" | "length" | "not_empty" | "cache_key" | "min"
            )
        });
        let written: Vec<(&str, &str, bool)> = found
            .markers
            .iter()
            .map(|marker| {
                (
                    marker.written.as_str(),
                    marker.arguments.as_str(),
                    marker.parameters.is_some(),
                )
            })
            .collect();
        assert_eq!(
            written,
            [
                ("required", "", false),
                ("length", "1, 80", false),
                ("not_empty", "", false),
                ("cache_key", "", true),
                ("min", "1", false),
            ]
        );
        let gap = |width: usize| " ".repeat(width);
        let expected = format!(
            "class Order:\n    title:{}str{}= ''\n    note: Annotated[str, Doc(\"x\"){}]\n    def find(self, key:{}str{}):\n        size:{}int{}= 1\n",
            gap(11),
            gap(27),
            gap(11),
            gap(11),
            gap(12),
            gap(11),
            gap(10)
        );
        assert_eq!(found.blanked(source), expected);
    }

    #[test]
    fn other_parameters_are_not_annotated_markers() {
        for code in ["order: Order", "order", "order: Annotated[Order, Meta]"] {
            assert!(PythonAnnotated::read(code, code, 0, |name| name == "from_body").is_none());
        }
    }
}
