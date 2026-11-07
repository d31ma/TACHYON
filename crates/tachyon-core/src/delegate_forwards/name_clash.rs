use super::{DelegateBase, DelegateForwards, DelegateRefusal, SourceLines};
use crate::Failure;
use std::path::Path;

pub(super) struct NameClash;

impl NameClash {
    const CODE: u16 = 1375;
    const FIELD_ENDS: [char; 5] = [':', ';', '=', ',', '{'];
    const NAME_MARKS: [char; 2] = ['!', '?'];
    const NOT_BEFORE: [&'static str; 4] = ["(", ".", "=", "return"];
    const METHOD_WORDS: [&'static str; 6] = ["def", "function", "fun", "func", "fn", "sub"];
    const GO: &'static str = "go";

    pub(super) fn check(
        path: &Path,
        (source, masked): (&str, &str),
        base: &DelegateBase<'_>,
        names: &[String],
    ) -> Result<(), Failure> {
        let language = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        let (line_start, _) = SourceLines::line_around(masked, base.field_at);
        let level = SourceLines::indentation(&masked[line_start..]);
        let lines = Self::field_lines(language, masked, line_start, level);
        let clash = names.iter().find_map(|name| {
            let at = Self::field_at(language, &lines, name)?;
            Some((name, at))
        });
        clash.map_or(Ok(()), |(name, at)| {
            Err(Self::failure(path, source, base, (name, at)))
        })
    }

    fn field_lines<'a>(
        language: &str,
        masked: &'a str,
        line_start: usize,
        level: &str,
    ) -> Vec<(usize, &'a str)> {
        let lines = SourceLines::with_offsets(masked);
        let field = lines
            .iter()
            .position(|(at, _)| *at == line_start)
            .unwrap_or_default();
        let above = lines[..field]
            .iter()
            .rev()
            .take_while(|(_, line)| Self::inside(line, level))
            .count();
        let below = lines[field..]
            .iter()
            .take_while(|(_, line)| Self::inside(line, level))
            .count();
        let struct_only = usize::from(language == Self::GO);
        let block = [0..lines.len(), field - above..field + below][struct_only].clone();
        lines[block]
            .iter()
            .copied()
            .filter(|(_, line)| SourceLines::indentation(line) == level)
            .collect()
    }

    fn inside(line: &str, level: &str) -> bool {
        let deeper = SourceLines::indentation(line).len() >= level.len();
        [SourceLines::is_blank(line), deeper].contains(&true)
    }

    fn field_at(language: &str, lines: &[(usize, &str)], name: &str) -> Option<usize> {
        lines.iter().find_map(|(at, line)| {
            let offset = Self::declared_at(language, line.trim_end(), name)?;
            Some(at + offset)
        })
    }

    fn declared_at(language: &str, line: &str, name: &str) -> Option<usize> {
        line.match_indices(name)
            .map(|(offset, _)| offset)
            .find(|offset| Self::declares_field(language, line, (*offset, name.len())))
    }

    fn declares_field(language: &str, line: &str, (offset, length): (usize, usize)) -> bool {
        let before = line[..offset].trim();
        let rest = &line[offset + length..];
        let whole = [line[..offset].chars().next_back(), rest.chars().next()]
            .into_iter()
            .flatten()
            .all(|letter| !SourceLines::is_word_letter(letter));
        let after = rest.trim_start_matches(Self::NAME_MARKS).trim_start();
        let ends = [after.is_empty(), after.starts_with(Self::FIELD_ENDS)].contains(&true);
        let go_field = [
            language == Self::GO,
            before.is_empty(),
            !after.starts_with('('),
        ] == [true, true, true];
        let plain = !Self::NOT_BEFORE.iter().any(|word| before.contains(word));
        let method = SourceLines::words(before)
            .iter()
            .any(|word| Self::METHOD_WORDS.contains(&word.as_str()));
        [whole, [ends, go_field].contains(&true), plain, !method] == [true, true, true, true]
    }

    fn failure(
        path: &Path,
        source: &str,
        base: &DelegateBase<'_>,
        (name, at): (&str, usize),
    ) -> Failure {
        let delegate = DelegateForwards::class_name(path);
        let service = &base.service;
        let message = format!(
            "{delegate} has a field named {name}, and its base {service} has a method named {name}. A Delegate takes on every public method of its base, so the two names clash."
        );
        let help = format!(
            "Rename the field, so that {name} on {delegate} stays the method that {service} offers (ADR 0112)."
        );
        DelegateRefusal { path, source }.at(Self::CODE, (at, name.len()), (message, help))
    }
}
