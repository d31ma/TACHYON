use super::{NeededField, UntypedField};
use std::ops::Range;

pub(crate) struct RubyNeeds;

impl RubyNeeds {
    const MARKERS: [&'static str; 2] = ["#[Needs]", "#[Needs()]"];
    const READER: &'static str = "attr_reader ";
    const COMMENT: char = '#';
    const SYMBOL: char = ':';
    const RETURN: &'static str = "# @return [";
    const TYPE_END: char = ']';

    pub(crate) fn typed(source: &str) -> Vec<NeededField> {
        let lines = Self::located_lines(source);
        (0..lines.len())
            .filter_map(|at| Self::typed_at(&lines, at))
            .collect()
    }

    fn located_lines(source: &str) -> Vec<(usize, &str)> {
        let mut lines = Vec::new();
        let mut start = 0;
        for line in source.split_inclusive('\n') {
            lines.push((start, line.trim_end()));
            start += line.len();
        }
        lines
    }

    pub(crate) fn untyped(source: &str) -> Vec<UntypedField> {
        let lines = Self::located_lines(source);
        (0..lines.len())
            .filter_map(|at| Self::untyped_at(&lines, at))
            .collect()
    }

    fn typed_at(lines: &[(usize, &str)], at: usize) -> Option<NeededField> {
        let (name, documentation, span) = Self::marked_reader(lines, at)?;
        let type_name = Self::documented_type(&documentation)?;
        Some(NeededField {
            name,
            type_name,
            span,
        })
    }

    fn untyped_at(lines: &[(usize, &str)], at: usize) -> Option<UntypedField> {
        let (name, documentation, span) = Self::marked_reader(lines, at)?;
        let typed = Self::documented_type(&documentation).is_some();
        (!typed).then_some(UntypedField { name, span })
    }

    fn marked_reader<'a>(
        lines: &[(usize, &'a str)],
        at: usize,
    ) -> Option<(String, Vec<&'a str>, Range<usize>)> {
        let (start, line) = lines[at];
        let symbols = line.trim().strip_prefix(Self::READER)?;
        let name = Self::names(symbols).into_iter().next()?;
        let documentation = Self::documentation_above(&lines[..at]);
        let marked = documentation
            .iter()
            .any(|text| Self::MARKERS.contains(text));
        marked.then(|| (name, documentation, start..start + line.len()))
    }

    fn documentation_above<'a>(lines: &[(usize, &'a str)]) -> Vec<&'a str> {
        lines
            .iter()
            .rev()
            .map(|(_, line)| line.trim())
            .take_while(|text| text.starts_with(Self::COMMENT))
            .collect()
    }

    fn documented_type(documentation: &[&str]) -> Option<String> {
        let documented = documentation
            .iter()
            .find_map(|text| text.strip_prefix(Self::RETURN))?;
        documented.strip_suffix(Self::TYPE_END).map(str::to_owned)
    }

    pub(super) fn fields(source: &str) -> Vec<String> {
        let lines: Vec<&str> = source.lines().map(str::trim).collect();
        (0..lines.len())
            .filter(|at| Self::MARKERS.contains(&lines[*at]))
            .filter_map(|at| Self::reader_after(&lines[at + 1..]))
            .flat_map(Self::names)
            .collect()
    }

    fn reader_after<'a>(lines: &[&'a str]) -> Option<&'a str> {
        let declaration = lines.iter().find(|line| !line.starts_with(Self::COMMENT))?;
        declaration.strip_prefix(Self::READER)
    }

    fn names(symbols: &str) -> Vec<String> {
        symbols
            .split(',')
            .filter_map(|part| part.trim().strip_prefix(Self::SYMBOL))
            .map(str::to_owned)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::RubyNeeds;

    #[test]
    fn a_needed_reader_is_found_below_its_marker_and_comments() {
        let source = "class GreetingService\n  #[Needs]\n  # @return [GreetingClient]\n  attr_reader :greeting_client\n\n  # @return [String]\n  attr_reader :message\nend\n";
        assert_eq!(RubyNeeds::fields(source), ["greeting_client"]);
    }

    #[test]
    fn a_needed_reader_keeps_its_type_whether_the_type_comes_before_or_after_the_marker() {
        let source = "class TallyService\n  #[Needs]\n  # @return [CounterStore]\n  attr_reader :store\n  # @return [CounterService]\n  #[Needs]\n  attr_reader :counter\n  # @return [Integer]\n  attr_reader :total\nend\n";
        let typed = RubyNeeds::typed(source);
        let found: Vec<(&str, &str, &str)> = typed
            .iter()
            .map(|field| {
                let text = &source[field.span.clone()];
                (field.name.as_str(), field.type_name.as_str(), text)
            })
            .collect();
        assert_eq!(
            found,
            [
                ("store", "CounterStore", "  attr_reader :store"),
                ("counter", "CounterService", "  attr_reader :counter"),
            ]
        );
    }

    #[test]
    fn a_needed_reader_without_a_return_type_is_untyped() {
        let source = "class TallyService\n  #[Needs]\n  attr_reader :counter\n  #[Needs]\n  # @return [CounterStore]\n  attr_reader :store\n  attr_reader :total\nend\n";
        let untyped = RubyNeeds::untyped(source);
        let found: Vec<(&str, &str)> = untyped
            .iter()
            .map(|field| (field.name.as_str(), &source[field.span.clone()]))
            .collect();
        assert_eq!(found, [("counter", "  attr_reader :counter")]);
    }
}
