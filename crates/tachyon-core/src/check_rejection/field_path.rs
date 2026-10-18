use super::PathSegment;
use std::fmt;

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct FieldPath {
    segments: Vec<PathSegment>,
}

impl FieldPath {
    pub(crate) const MAX_TEXT_BYTES: usize = 512;

    #[cfg(test)]
    pub(crate) fn root() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) fn field(mut self, name: &str) -> Self {
        self.segments.push(PathSegment::Name(name.to_owned()));
        self
    }

    #[cfg(test)]
    pub(crate) fn item(mut self, index: u32) -> Self {
        self.segments.push(PathSegment::Item(index));
        self
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        let text = Self::within_limit(text)?;
        let dotted = Self::with_leading_dot(text);
        let segments = Self::read_segments(&dotted)?;
        Some(Self { segments })
    }

    fn within_limit(text: &str) -> Option<&str> {
        if text.len() > Self::MAX_TEXT_BYTES {
            return None;
        }
        Some(text)
    }

    fn with_leading_dot(text: &str) -> String {
        if matches!(text.chars().next(), None | Some('[')) {
            return text.to_owned();
        }
        format!(".{text}")
    }

    fn read_segments(text: &str) -> Option<Vec<PathSegment>> {
        let mut segments = Vec::new();
        let mut rest = text;
        while !rest.is_empty() {
            let (segment, after) = Self::next_segment(rest)?;
            segments.push(segment);
            rest = after;
        }
        Some(segments)
    }

    fn next_segment(text: &str) -> Option<(PathSegment, &str)> {
        if let Some(inside) = text.strip_prefix('[') {
            return Self::read_item(inside);
        }
        let name = text.strip_prefix('.')?;
        Self::read_name(name)
    }

    fn read_name(text: &str) -> Option<(PathSegment, &str)> {
        let length = text
            .bytes()
            .take_while(|byte| Self::name_byte(*byte))
            .count();
        let (name, rest) = text.split_at(length);
        let starts_well = name.bytes().next().is_some_and(Self::first_name_byte);
        starts_well.then(|| (PathSegment::Name(name.to_owned()), rest))
    }

    fn read_item(text: &str) -> Option<(PathSegment, &str)> {
        let (digits, rest) = text.split_once(']')?;
        let index = Self::canonical_index(digits)?;
        Some((PathSegment::Item(index), rest))
    }

    fn canonical_index(digits: &str) -> Option<u32> {
        let index = digits.parse::<u32>().ok()?;
        (index.to_string() == digits).then_some(index)
    }

    fn name_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_', byte == b'$'].contains(&true)
    }

    fn first_name_byte(byte: u8) -> bool {
        Self::name_byte(byte) && !byte.is_ascii_digit()
    }

    fn write_all(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, segment) in self.segments.iter().enumerate() {
            Self::write_segment(formatter, index, segment)?;
        }
        Ok(())
    }

    fn write_segment(
        formatter: &mut fmt::Formatter<'_>,
        index: usize,
        segment: &PathSegment,
    ) -> fmt::Result {
        match segment {
            PathSegment::Name(name) if index == 0 => write!(formatter, "{name}"),
            PathSegment::Name(name) => write!(formatter, ".{name}"),
            PathSegment::Item(item) => write!(formatter, "[{item}]"),
        }
    }
}

impl fmt::Display for FieldPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_all(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::FieldPath;

    #[test]
    fn a_built_path_reaches_nested_fields_and_array_items() {
        let path = FieldPath::root().field("items").item(2).field("name");
        assert_eq!(path.to_string(), "items[2].name");
        assert_eq!(FieldPath::root().field("name").to_string(), "name");
        assert_eq!(FieldPath::root().item(0).to_string(), "[0]");
        assert_eq!(FieldPath::root().to_string(), "");
        let deep = FieldPath::root().field("a").field("b").item(10).item(3);
        assert_eq!(deep.to_string(), "a.b[10][3]");
    }

    #[test]
    fn a_path_reads_back_from_its_text() {
        for text in [
            "",
            "name",
            "items[2].name",
            "a.b[10][3]",
            "$id",
            "_x.y_2",
            "[0]",
        ] {
            let path = FieldPath::parse(text);
            assert_eq!(path.map(|path| path.to_string()).as_deref(), Some(text));
        }
    }

    #[test]
    fn malformed_paths_are_refused() {
        let malformed = [
            ".name",
            "name.",
            "a..b",
            "2name",
            "items[]",
            "items[02]",
            "items[-1]",
            "items[x]",
            "items[1",
            "items]",
            "na me",
            "name\"",
            "items[4294967296]",
            "näme",
        ];
        for text in malformed {
            assert_eq!(FieldPath::parse(text), None, "{text}");
        }
        let long = "a".repeat(FieldPath::MAX_TEXT_BYTES + 1);
        assert_eq!(FieldPath::parse(&long), None);
        let longest = "a".repeat(FieldPath::MAX_TEXT_BYTES);
        assert!(FieldPath::parse(&longest).is_some());
    }

    #[test]
    fn paths_sort_by_segment_with_numeric_items() {
        let mut paths: Vec<FieldPath> = [
            "items[10]",
            "name",
            "items[2].name",
            "items",
            "items[2]",
            "age",
        ]
        .into_iter()
        .filter_map(FieldPath::parse)
        .collect();
        paths.sort();
        let texts: Vec<String> = paths.iter().map(ToString::to_string).collect();
        assert_eq!(
            texts,
            [
                "age",
                "items",
                "items[2]",
                "items[2].name",
                "items[10]",
                "name"
            ]
        );
    }
}
