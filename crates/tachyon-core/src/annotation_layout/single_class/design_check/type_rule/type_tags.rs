pub(super) struct TypeTags {
    names: Vec<String>,
    returns: bool,
    typed: bool,
}

impl TypeTags {
    const PARAM: &'static str = "@param";
    const SCRIPT_RETURNS: [&'static str; 2] = ["@returns", "@return"];
    const YARD_RETURNS: [&'static str; 1] = ["@return"];
    const TYPE: &'static str = "@type";
    const OPEN: char = '{';
    const CLOSE: char = '}';
    const YARD_OPEN: char = '[';
    const OPTIONAL: char = '[';
    const NAME_MARKS: &'static str = "_$";

    pub(super) fn js_doc(comments: &str) -> Self {
        Self {
            names: Self::param_names(comments, Self::js_doc_name),
            returns: Self::has_any_tag(comments, &Self::SCRIPT_RETURNS, Self::OPEN),
            typed: Self::has_tag(comments, Self::TYPE, Self::OPEN),
        }
    }

    pub(super) fn yard(comments: &str) -> Self {
        Self {
            names: Self::param_names(comments, Self::yard_name),
            returns: Self::has_any_tag(comments, &Self::YARD_RETURNS, Self::YARD_OPEN),
            typed: false,
        }
    }

    pub(super) fn names(&self, name: &str) -> bool {
        self.names.iter().any(|known| known == name)
    }

    pub(super) const fn returns(&self) -> bool {
        self.returns
    }

    pub(super) const fn typed(&self) -> bool {
        self.typed
    }

    fn param_names(comments: &str, name: fn(&str) -> Option<String>) -> Vec<String> {
        comments
            .match_indices(Self::PARAM)
            .filter_map(|(at, _)| name(&comments[at + Self::PARAM.len()..]))
            .collect()
    }

    fn js_doc_name(rest: &str) -> Option<String> {
        let typed = rest.trim_start().strip_prefix(Self::OPEN)?;
        let close = Self::type_end(typed)?;
        let named = typed[close + 1..]
            .trim_start()
            .trim_start_matches(Self::OPTIONAL);
        Some(Self::leading_name(named)).filter(|name| !name.is_empty())
    }

    fn yard_name(rest: &str) -> Option<String> {
        let named = rest.trim_start();
        let name = Self::leading_name(named);
        let typed = named[name.len()..]
            .trim_start()
            .starts_with(Self::YARD_OPEN);
        typed.then_some(name)
    }

    fn leading_name(text: &str) -> String {
        text.chars()
            .take_while(|&character| Self::is_name_character(character))
            .collect()
    }

    fn is_name_character(character: char) -> bool {
        Self::NAME_MARKS.contains(character) | character.is_alphanumeric()
    }

    fn type_end(typed: &str) -> Option<usize> {
        typed
            .char_indices()
            .scan(0_isize, |depth, (at, character)| {
                *depth += Self::depth_change(character);
                Some((at, *depth))
            })
            .find(|(_, depth)| *depth < 0)
            .map(|(at, _)| at)
    }

    fn depth_change(character: char) -> isize {
        isize::from(character == Self::OPEN) - isize::from(character == Self::CLOSE)
    }

    fn has_any_tag(comments: &str, tags: &[&str], open: char) -> bool {
        tags.iter().any(|tag| Self::has_tag(comments, tag, open))
    }

    fn has_tag(comments: &str, tag: &str, open: char) -> bool {
        comments
            .match_indices(tag)
            .any(|(at, _)| comments[at + tag.len()..].trim_start().starts_with(open))
    }
}
