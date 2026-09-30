use super::contract_declaration::ContractDeclaration;
use super::contract_form::ContractForm;
use super::contract_member::ContractMember;
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryBase {
    pub(crate) name_at: usize,
    pub(crate) clause: Range<usize>,
    pub(crate) entity: String,
    pub(crate) id: String,
}

impl RepositoryBase {
    pub(crate) const NAME: &'static str = "Repository";
    const GO: &'static str = "go";
    const PYTHON: &'static str = "py";
    const KEYWORDS: [&'static str; 4] = ["extends", "implements", ":", "<"];
    const GENERIC_OPENERS: [u8; 2] = *b"<[";
    const OPENERS: [u8; 3] = *b"<[(";
    const CLOSERS: [u8; 3] = *b">])";
    const LIST_SEPARATOR: char = ',';

    pub(crate) fn find(
        form: ContractForm,
        code: &str,
        declaration: &ContractDeclaration,
    ) -> Option<Self> {
        let embedded = form.language == Self::GO;
        let areas = [&declaration.header, &declaration.body];
        let area = areas[usize::from(embedded)].clone();
        let at = Self::word_at(code, area.clone())?;
        let end = Self::generic_end(code, at + Self::NAME.len());
        let (entity, id) = Self::arguments(&code[at + Self::NAME.len()..end]);
        let clause = if embedded {
            Self::line_of(code, at)
        } else if form.language == Self::PYTHON {
            Self::listed(code, at, end)
        } else {
            Self::keyword_start(code, area.start, at)..end
        };
        Some(Self {
            name_at: at,
            clause,
            entity,
            id,
        })
    }

    pub(crate) fn is_embedded(form: ContractForm, code: &str, member: &ContractMember) -> bool {
        let text = code[member.start..member.end].trim_start();
        let named = Self::starts_with_word(text);
        [form.language == Self::GO, member.method, named] == [true, false, true]
    }

    pub(crate) fn without_clause(&self, text: &str, offset: usize) -> String {
        let start = self.clause.start.saturating_sub(offset).min(text.len());
        let end = self.clause.end.saturating_sub(offset).min(text.len());
        format!("{}{}", &text[..start], &text[end..])
    }

    fn starts_with_word(text: &str) -> bool {
        text.strip_prefix(Self::NAME)
            .is_some_and(|rest| !rest.starts_with(Self::is_name_char))
    }

    fn is_name_char(character: char) -> bool {
        [character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }

    fn word_at(code: &str, area: Range<usize>) -> Option<usize> {
        code[area.clone()]
            .match_indices(Self::NAME)
            .map(|(at, _)| area.start + at)
            .find(|at| Self::is_whole_word(code, *at))
    }

    fn is_whole_word(code: &str, at: usize) -> bool {
        let before = code[..at]
            .chars()
            .next_back()
            .is_some_and(Self::is_name_char);
        let after = code[at + Self::NAME.len()..]
            .chars()
            .next()
            .is_some_and(Self::is_name_char);
        [before, after] == [false, false]
    }

    fn generic_end(code: &str, after: usize) -> usize {
        let opens = code
            .as_bytes()
            .get(after)
            .is_some_and(|byte| Self::GENERIC_OPENERS.contains(byte));
        [after, Self::group_end(code, after)][usize::from(opens)]
    }

    fn group_end(code: &str, open: usize) -> usize {
        code.bytes()
            .enumerate()
            .skip(open)
            .scan(0_i32, |depth, (at, byte)| {
                *depth += i32::from(Self::OPENERS.contains(&byte))
                    - i32::from(Self::CLOSERS.contains(&byte));
                Some((*depth, at))
            })
            .find(|(depth, _)| *depth == 0)
            .map_or(code.len(), |(_, at)| at + 1)
    }

    fn arguments(generics: &str) -> (String, String) {
        let inner = generics
            .get(1..generics.len().saturating_sub(1))
            .unwrap_or("");
        let comma = Self::top_level_comma(inner).unwrap_or(inner.len());
        let entity = inner[..comma].trim().to_owned();
        let id = inner.get(comma + 1..).unwrap_or("").trim().to_owned();
        (entity, id)
    }

    fn top_level_comma(text: &str) -> Option<usize> {
        text.bytes()
            .enumerate()
            .scan(0_i32, |depth, (at, byte)| {
                *depth += i32::from(Self::OPENERS.contains(&byte))
                    - i32::from(Self::CLOSERS.contains(&byte));
                Some((*depth, at, byte))
            })
            .find(|(depth, _, byte)| [*depth == 0, *byte == b','] == [true, true])
            .map(|(_, at, _)| at)
    }

    fn line_of(code: &str, at: usize) -> Range<usize> {
        let start = code[..at].rfind('\n').map_or(0, |line| line + 1);
        let end = code[at..]
            .find('\n')
            .map_or(code.len(), |line| at + line + 1);
        start..end
    }

    fn keyword_start(code: &str, header_start: usize, at: usize) -> usize {
        let before = code[header_start..at].trim_end();
        let without_keyword = Self::KEYWORDS
            .iter()
            .find_map(|keyword| before.strip_suffix(keyword))
            .unwrap_or(before);
        header_start + without_keyword.trim_end().len()
    }

    fn listed(code: &str, at: usize, end: usize) -> Range<usize> {
        let rest = &code[end..];
        let following = rest.trim_start();
        let separated = following.strip_prefix(Self::LIST_SEPARATOR).map(|after| {
            let spaces = after.len() - after.trim_start().len();
            end + (rest.len() - following.len()) + 1 + spaces
        });
        separated.map_or_else(
            || {
                let before = code[..at].trim_end();
                let start = before
                    .strip_suffix(Self::LIST_SEPARATOR)
                    .map_or(at, str::len);
                start..end
            },
            |after| at..after,
        )
    }
}
