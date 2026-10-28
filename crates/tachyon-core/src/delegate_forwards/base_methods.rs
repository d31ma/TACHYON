use super::{DelegateBase, SourceLines};
use crate::method_coverage::RoleClass;
use std::collections::BTreeSet;

pub(super) struct BaseMethods;

impl BaseMethods {
    const ACCESSORS: [&'static str; 2] = ["get ", "set "];
    const NOT_TAKEN_ON: [&'static str; 6] = [
        "onstart", "onstop", "onevent", "property", "setter", "getter",
    ];

    pub(super) fn listed(base: &DelegateBase<'_>) -> Vec<(String, usize)> {
        let class = RoleClass::read(base.service_path, base.service_source)
            .ok()
            .flatten();
        let lines: Vec<&str> = class
            .as_ref()
            .map(|class| class.masked.lines().collect())
            .unwrap_or_default();
        let source_lines: Vec<&str> = base.service_source.lines().collect();
        let methods: Vec<(&str, usize)> = class
            .iter()
            .flat_map(|class| class.methods.iter())
            .map(|method| (method.name.as_str(), method.line))
            .collect();
        let level = Self::member_level(&lines, &methods);
        let mut seen = BTreeSet::new();
        methods
            .into_iter()
            .filter(|(_, line)| Self::indentation(&lines, *line) == level)
            .filter(|(_, line)| !Self::is_accessor(&lines, *line))
            .filter(|(_, line)| !Self::is_not_taken_on(&source_lines, *line))
            .filter(|(name, _)| seen.insert(*name))
            .map(|(name, line)| (name.to_owned(), line))
            .collect()
    }

    fn member_level(lines: &[&str], methods: &[(&str, usize)]) -> usize {
        methods
            .iter()
            .map(|(_, line)| Self::indentation(lines, *line))
            .min()
            .unwrap_or_default()
    }

    fn indentation(lines: &[&str], line: usize) -> usize {
        lines
            .get(line)
            .map_or(0, |text| SourceLines::indentation(text).len())
    }

    fn is_accessor(lines: &[&str], line: usize) -> bool {
        let text = lines.get(line).map_or("", |text| text.trim_start());
        Self::ACCESSORS
            .iter()
            .any(|accessor| text.starts_with(accessor))
    }

    fn is_not_taken_on(source_lines: &[&str], line: usize) -> bool {
        let notes = source_lines[..line.min(source_lines.len())]
            .iter()
            .rev()
            .take_while(|text| SourceLines::is_note(text));
        notes
            .flat_map(|text| SourceLines::words(text))
            .map(|word| word.replace('_', "").to_ascii_lowercase())
            .any(|word| Self::NOT_TAKEN_ON.contains(&word.as_str()))
    }
}
