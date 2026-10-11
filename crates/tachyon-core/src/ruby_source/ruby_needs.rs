pub(super) struct RubyNeeds;

impl RubyNeeds {
    const MARKERS: [&'static str; 2] = ["#[Needs]", "#[Needs()]"];
    const READER: &'static str = "attr_reader ";
    const COMMENT: char = '#';
    const SYMBOL: char = ':';

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
}
