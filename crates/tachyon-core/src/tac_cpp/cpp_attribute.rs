use super::CppTokens;
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CppAttribute {
    pub(crate) name: String,
    pub(crate) arguments: String,
    pub(crate) at: usize,
    pub(crate) specifier: Range<usize>,
}

impl CppAttribute {
    pub(crate) fn has_arguments(&self) -> bool {
        !self.arguments.trim().is_empty()
    }
}

pub(crate) struct CppAttributes;

impl CppAttributes {
    pub(crate) fn read(
        tokens: &CppTokens,
        source: &str,
        mut at: usize,
    ) -> (Vec<CppAttribute>, usize) {
        let mut found = Vec::new();
        while tokens.opens_attribute(at) {
            let (read, next) = Self::specifier(tokens, source, at);
            found.extend(read);
            at = next;
        }
        (found, at)
    }

    fn specifier(tokens: &CppTokens, source: &str, open: usize) -> (Vec<CppAttribute>, usize) {
        let outer_close = tokens.partner(open).unwrap_or(tokens.len());
        let inner_close = tokens.partner(open + 1).unwrap_or(outer_close);
        let range = tokens.start(open)..tokens.end(outer_close).min(source.len());
        let items = Self::items(tokens, open + 2, inner_close);
        let attributes = items
            .into_iter()
            .filter_map(|item| Self::attribute(tokens, source, item, &range))
            .collect();
        (attributes, outer_close + 1)
    }

    fn items(tokens: &CppTokens, mut at: usize, end: usize) -> Vec<Range<usize>> {
        let mut items = Vec::new();
        let mut start = at;
        while at < end {
            let comma = tokens.is(at, ",");
            items.extend(comma.then_some(start..at));
            start = if comma { at + 1 } else { start };
            at = tokens
                .partner(at)
                .filter(|partner| *partner > at)
                .map_or(at, |partner| partner)
                + 1;
        }
        items.push(start..end);
        items
    }

    fn attribute(
        tokens: &CppTokens,
        source: &str,
        item: Range<usize>,
        specifier: &Range<usize>,
    ) -> Option<CppAttribute> {
        let name_end = Self::name_end(tokens, item.clone());
        let name: String = (item.start..name_end)
            .map(|index| tokens.text(index))
            .collect();
        let arguments = Self::arguments(tokens, source, name_end, item.end);
        (!name.is_empty()).then(|| CppAttribute {
            name,
            arguments,
            at: tokens.start(item.start),
            specifier: specifier.clone(),
        })
    }

    fn name_end(tokens: &CppTokens, item: Range<usize>) -> usize {
        (item.start..item.end)
            .find(|index| tokens.is(*index, "("))
            .unwrap_or(item.end)
    }

    fn arguments(tokens: &CppTokens, source: &str, open: usize, end: usize) -> String {
        let close = tokens
            .partner(open)
            .filter(|close| tokens.is(open, "(") && *close < end);
        close.map_or_else(String::new, |close| {
            source[tokens.end(open)..tokens.start(close)]
                .trim()
                .to_owned()
        })
    }
}
