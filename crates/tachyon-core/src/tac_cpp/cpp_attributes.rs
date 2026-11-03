use super::{CppAttribute, CppTokens};
use std::ops::Range;

pub(crate) struct CppAttributes;

impl CppAttributes {
    pub(crate) fn read(tokens: &CppTokens, source: &str, at: usize) -> (Vec<CppAttribute>, usize) {
        Self::specifiers_from(tokens, source, at)
    }

    fn specifiers_from(
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
        let attributes = Self::items(tokens, open + 2, inner_close)
            .into_iter()
            .filter_map(|item| Self::attribute(tokens, source, item, &range))
            .collect();
        (attributes, outer_close + 1)
    }

    fn items(tokens: &CppTokens, start: usize, end: usize) -> Vec<Range<usize>> {
        let mut cuts = Self::commas(tokens, start, end);
        cuts.push(end);
        let mut first = start;
        let mut items = Vec::new();
        for cut in cuts {
            items.push(first..cut);
            first = cut + 1;
        }
        items
    }

    fn commas(tokens: &CppTokens, start: usize, end: usize) -> Vec<usize> {
        let mut found = Vec::new();
        let mut at = start;
        while at < end {
            found.extend(tokens.is(at, ",").then_some(at));
            at = tokens.skipped(at);
        }
        found
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
            .filter(|close| [tokens.is(open, "("), *close < end] == [true, true]);
        close.map_or_else(String::new, |close| {
            source[tokens.end(open)..tokens.start(close)]
                .trim()
                .to_owned()
        })
    }
}
