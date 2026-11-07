pub(super) struct ModelText;

impl ModelText {
    pub(super) fn mentioned<'n>(text: &str, names: &[&'n str]) -> Vec<&'n str> {
        names
            .iter()
            .copied()
            .filter(|name| Self::mentions(text, name))
            .collect()
    }

    pub(super) fn mentions(text: &str, name: &str) -> bool {
        text.match_indices(name)
            .any(|(at, _)| Self::is_whole_word(text, at, at + name.len()))
    }

    pub(super) fn import_lines(extension: &str, names: &[&str]) -> String {
        names
            .iter()
            .filter_map(|name| Self::import_line(extension, name))
            .collect()
    }

    pub(super) fn merged(ranges: &[(usize, usize)]) -> Vec<(usize, usize)> {
        let mut sorted = ranges.to_vec();
        sorted.sort_unstable();
        sorted.into_iter().fold(Vec::new(), Self::merge_one)
    }

    fn merge_one(mut merged: Vec<(usize, usize)>, range: (usize, usize)) -> Vec<(usize, usize)> {
        match merged.last_mut() {
            Some(last) if range.0 <= last.1 => last.1 = last.1.max(range.1),
            _ => merged.push(range),
        }
        merged
    }

    fn import_line(extension: &str, name: &str) -> Option<String> {
        match extension {
            "ts" => Some(format!("import type {{ {name} }} from './{name}.ts'\n")),
            "rs" => Some(format!("use crate::models::{name};\n")),
            "dart" => Some(format!("import '{name}.dart';\n")),
            _ => None,
        }
    }

    fn is_whole_word(text: &str, start: usize, end: usize) -> bool {
        let bytes = text.as_bytes();
        let before = start.checked_sub(1).and_then(|at| bytes.get(at)).copied();
        let after = bytes.get(end).copied();
        ![before, after]
            .into_iter()
            .flatten()
            .any(Self::is_name_byte)
    }

    pub(super) fn is_name_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_', byte == b'$'].contains(&true)
    }
}
