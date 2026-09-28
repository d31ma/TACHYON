#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TypeImport {
    pub(super) start: usize,
    pub(super) end: usize,
    keyword: &'static str,
    names: Vec<String>,
    quote: char,
    pub(super) spec: String,
    ending: &'static str,
}

impl TypeImport {
    const IMPORT: &'static str = "import ";
    const TYPE_IMPORT: &'static str = "import type ";
    const TYPE: &'static str = "type ";
    const FROM: &'static str = "from";
    const QUOTES: [char; 2] = ['\'', '"'];
    const SEMICOLON: &'static str = ";";

    pub(super) fn all(source: &str) -> Vec<Self> {
        source
            .match_indices(Self::IMPORT)
            .filter(|(at, _)| Self::starts_line(source, *at))
            .filter_map(|(at, _)| Self::parse(source, at))
            .collect()
    }

    pub(super) fn imports(&self, name: &str) -> bool {
        self.names.iter().any(|written| Self::base(written) == name)
    }

    pub(super) fn split(&self, moved: &[&str], new_spec: &dyn Fn(&str) -> String) -> String {
        let (going, staying): (Vec<&String>, Vec<&String>) = self
            .names
            .iter()
            .partition(|written| moved.contains(&Self::base(written)));
        let kept = self.line_for(&staying, &self.spec);
        let added = going
            .into_iter()
            .map(|written| self.line_for(&[written], &new_spec(Self::base(written))));
        kept.into_iter()
            .chain(added.flatten())
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn starts_line(source: &str, at: usize) -> bool {
        let line_start = source[..at].rfind('\n').map_or(0, |index| index + 1);
        source[line_start..at].trim().is_empty()
    }

    fn parse(source: &str, start: usize) -> Option<Self> {
        let rest = &source[start + Self::IMPORT.len()..];
        let type_only = rest.starts_with(Self::TYPE);
        let rest = rest.strip_prefix(Self::TYPE).unwrap_or(rest).trim_start();
        let inner = rest.strip_prefix('{')?;
        let close = inner.find('}')?;
        let after = inner[close + 1..]
            .trim_start()
            .strip_prefix(Self::FROM)?
            .trim_start();
        let quote = after
            .chars()
            .next()
            .filter(|first| Self::QUOTES.contains(first))?;
        let spec_length = after[1..].find(quote)?;
        let spec = &after[1..=spec_length];
        let tail = &after[spec_length + 2..];
        let ending = ["", Self::SEMICOLON][usize::from(tail.starts_with(Self::SEMICOLON))];
        let end = source.len() - tail.len() + ending.len();
        Some(Self {
            start,
            end,
            keyword: [Self::IMPORT, Self::TYPE_IMPORT][usize::from(type_only)],
            names: Self::names(&inner[..close]),
            quote,
            spec: spec.to_owned(),
            ending,
        })
    }

    fn names(list: &str) -> Vec<String> {
        list.split(',')
            .map(str::trim)
            .filter(|written| !written.is_empty())
            .map(str::to_owned)
            .collect()
    }

    fn base(written: &str) -> &str {
        let written = written.strip_prefix(Self::TYPE).unwrap_or(written);
        written.split_whitespace().next().unwrap_or(written)
    }

    fn line_for(&self, names: &[&String], spec: &str) -> Option<String> {
        let list: Vec<&str> = names.iter().map(|name| name.as_str()).collect();
        (!list.is_empty()).then(|| {
            format!(
                "{}{{ {} }} from {}{spec}{}{}",
                self.keyword,
                list.join(", "),
                self.quote,
                self.quote,
                self.ending
            )
        })
    }
}
