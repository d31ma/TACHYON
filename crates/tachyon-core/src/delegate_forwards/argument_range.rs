use super::{ParameterSplit, SourceLines};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ArgumentRange {
    pub(super) required: usize,
    pub(super) most: Option<usize>,
}

impl ArgumentRange {
    const SKIPPED: [(&'static str, &'static str); 6] = [
        ("py", "self"),
        ("py", "cls"),
        ("py", "*"),
        ("py", "/"),
        ("ts", "this"),
        ("rb", "&"),
    ];
    const REST_STARTS: [(&'static str, &'static str); 6] = [
        ("js", "..."),
        ("ts", "..."),
        ("php", "..."),
        ("lua", "..."),
        ("py", "*"),
        ("rb", "*"),
    ];

    pub(super) fn read(
        language: &str,
        (source, masked): (&str, &str),
        start: usize,
        name: &str,
    ) -> Option<Self> {
        let (_, line_end) = SourceLines::line_around(masked, start);
        let line = &masked[start..line_end];
        let name_end = start + Self::name_offset(line, name)? + name.len();
        let open = masked[name_end..line_end]
            .trim_start()
            .starts_with('(')
            .then(|| name_end + masked[name_end..].find('('))
            .flatten();
        let inside = open.map_or(Some(name_end..line_end), |open| {
            Some(open + 1..ParameterSplit::matching_close(masked, open)?)
        })?;
        let parts = ParameterSplit::parts(source, masked, inside);
        Some(Self::counted(language, &parts))
    }

    pub(super) fn name_offset(line: &str, name: &str) -> Option<usize> {
        line.match_indices(name)
            .map(|(offset, _)| offset)
            .find(|offset| Self::whole_word(line, *offset, name.len()))
    }

    pub(super) fn overlaps(self, other: Self) -> bool {
        let own_before = self.most.is_some_and(|most| most < other.required);
        let other_before = other.most.is_some_and(|most| most < self.required);
        [own_before, other_before] == [false, false]
    }

    pub(super) fn shown(self) -> String {
        let required = self.required;
        self.most.map_or_else(
            || format!("{required} or more"),
            |most| {
                (most == required)
                    .then(|| required.to_string())
                    .unwrap_or_else(|| format!("{required} to {most}"))
            },
        )
    }

    fn whole_word(line: &str, offset: usize, length: usize) -> bool {
        let before = line[..offset].chars().next_back();
        let after = line[offset + length..].chars().next();
        [before, after]
            .into_iter()
            .flatten()
            .all(|letter| !SourceLines::is_word_letter(letter))
    }

    fn counted(language: &str, parts: &[String]) -> Self {
        let kept: Vec<&String> = parts
            .iter()
            .filter(|part| !Self::is_skipped(language, part))
            .collect();
        let rest = kept.iter().any(|part| Self::is_rest(language, part));
        let required = kept
            .iter()
            .filter(|part| !Self::is_rest(language, part))
            .filter(|part| !Self::is_optional(language, part))
            .count();
        let named = kept
            .iter()
            .filter(|part| !Self::is_rest(language, part))
            .count();
        Self {
            required,
            most: (!rest).then_some(named),
        }
    }

    fn is_skipped(language: &str, part: &str) -> bool {
        let word = part
            .split(|letter: char| [':', '=', ' '].contains(&letter))
            .next()
            .unwrap_or_default();
        let block = [language == "rb", part.starts_with('&')] == [true, true];
        [Self::SKIPPED.contains(&(language, word)), block].contains(&true)
    }

    fn is_rest(language: &str, part: &str) -> bool {
        let started = Self::REST_STARTS
            .iter()
            .filter(|(owner, _)| *owner == language)
            .any(|(_, start)| part.starts_with(start));
        let php_spread = [language == "php", part.contains("...")] == [true, true];
        [started, php_spread].contains(&true)
    }

    fn is_optional(language: &str, part: &str) -> bool {
        let defaulted = part.contains('=');
        let typescript_optional = [language == "ts", Self::marked_optional(part)] == [true, true];
        let ruby_keyword_default = [language == "rb", Self::ruby_default(part)] == [true, true];
        [defaulted, typescript_optional, ruby_keyword_default].contains(&true)
    }

    fn marked_optional(part: &str) -> bool {
        part.split(':')
            .next()
            .is_some_and(|name| name.trim_end().ends_with('?'))
    }

    fn ruby_default(part: &str) -> bool {
        part.split_once(':')
            .is_some_and(|(_, value)| !value.trim().is_empty())
    }
}
