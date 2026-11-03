#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;
use std::ops::Range;

pub(super) struct NamePicker;

impl NamePicker {
    const ANGLED: [&'static str; 7] = ["ts", "java", "cs", "kt", "swift", "rs", "dart"];
    const ARROWS: [&'static str; 2] = ["->", "=>"];
    const COLON_FIRST: [&'static str; 5] = ["ts", "kt", "swift", "rs", "py"];
    const TYPE_FIRST: [&'static str; 3] = ["java", "cs", "dart"];
    const GROUPED: [&'static str; 2] = ["{", "["];
    const MODIFIERS: [&'static str; 22] = [
        "mut",
        "ref",
        "out",
        "in",
        "params",
        "readonly",
        "public",
        "private",
        "protected",
        "final",
        "const",
        "var",
        "val",
        "let",
        "vararg",
        "required",
        "override",
        "static",
        "late",
        "internal",
        "open",
        "inout",
    ];
    const DOLLAR: &'static str = "$";
    const PHP: &'static str = "php";

    pub(super) fn split_at_commas(class: &ClassFile, range: Range<usize>) -> Vec<Range<usize>> {
        let cuts = Self::comma_cuts(class, range.clone());
        let starts = std::iter::once(range.start).chain(cuts.iter().map(|cut| cut + 1));
        let ends = cuts.iter().copied().chain(std::iter::once(range.end));
        starts.zip(ends).map(|(start, end)| start..end).collect()
    }

    pub(super) fn parameter_name(class: &ClassFile, part: Range<usize>) -> Option<usize> {
        let kept = Self::without_markers(class, part);
        let dollar = Self::after_dollar(class, &kept);
        let typed = Self::typed_name(class, &kept);
        dollar.or(typed)
    }

    pub(super) fn field_name(class: &ClassFile, kept: &[usize]) -> Option<usize> {
        let dollar = Self::after_dollar(class, kept);
        let colon = Self::before_colon(class, kept);
        let assigned = Self::last_name_before(class, kept, Self::equals_at(class, kept));
        dollar.or(colon).or(assigned)
    }

    pub(super) fn without_markers(class: &ClassFile, range: Range<usize>) -> Vec<usize> {
        Self::outside_markers(class, range)
    }

    fn outside_markers(class: &ClassFile, range: Range<usize>) -> Vec<usize> {
        let mut kept = Vec::new();
        let mut at = range.start;
        while at < range.end {
            let end = class.tokens.marker_end(at).unwrap_or(at);
            kept.extend((end == at).then_some(at));
            at = end.max(at + 1);
        }
        kept
    }

    fn comma_cuts(class: &ClassFile, range: Range<usize>) -> Vec<usize> {
        let base = class.words.depth(range.start);
        let generic_language = Self::ANGLED.contains(&class.extension);
        let mut angles = 0_isize;
        let mut cuts = Vec::new();
        for at in range {
            let level = class.words.depth(at) == base;
            angles = Self::angle_depth(class, at, angles, generic_language & level);
            let cut = level & (angles == 0) & (class.words.text(at) == ",");
            cuts.extend(cut.then_some(at));
        }
        cuts
    }

    fn angle_depth(class: &ClassFile, at: usize, angles: isize, counted: bool) -> isize {
        let text = class.words.text(at);
        let arrow = Self::ARROWS.contains(&class.words.pair(at.wrapping_sub(1)));
        let opens = counted & (text == "<");
        let closes = counted & (text == ">") & !arrow;
        (angles + isize::from(opens) - isize::from(closes)).max(0)
    }

    fn typed_name(class: &ClassFile, kept: &[usize]) -> Option<usize> {
        let colon_first = Self::COLON_FIRST.contains(&class.extension);
        let type_first = Self::TYPE_FIRST.contains(&class.extension);
        let colon = Self::before_colon(class, kept).filter(|_| colon_first);
        let last = Self::last_name_before(class, kept, Self::equals_at(class, kept))
            .filter(|_| type_first);
        colon.or(last).or_else(|| Self::first_name(class, kept))
    }

    fn after_dollar(class: &ClassFile, kept: &[usize]) -> Option<usize> {
        let php = class.extension == Self::PHP;
        let dollar = kept
            .iter()
            .position(|&at| class.words.text(at) == Self::DOLLAR)
            .filter(|_| php)?;
        kept.get(dollar + 1).copied()
    }

    fn before_colon(class: &ClassFile, kept: &[usize]) -> Option<usize> {
        let base = kept.first().map_or(0, |&at| class.words.depth(at));
        let stop = Self::equals_at(class, kept);
        let colon = kept[..stop]
            .iter()
            .position(|&at| (class.words.text(at) == ":") & (class.words.depth(at) == base))?;
        Self::last_name_before(class, kept, colon)
    }

    pub(super) fn first_plain_word<'c>(class: &'c ClassFile, kept: &[usize]) -> &'c str {
        kept.iter()
            .map(|&at| class.words.text(at))
            .find(|text| !Self::MODIFIERS.contains(text))
            .unwrap_or("")
    }

    pub(super) fn equals_at(class: &ClassFile, kept: &[usize]) -> usize {
        let base = kept.first().map_or(0, |&at| class.words.depth(at));
        kept.iter()
            .position(|&at| (class.words.text(at) == "=") & (class.words.depth(at) == base))
            .unwrap_or(kept.len())
    }

    fn last_name_before(class: &ClassFile, kept: &[usize], stop: usize) -> Option<usize> {
        let base = kept.first().map_or(0, |&at| class.words.depth(at));
        kept[..stop.min(kept.len())]
            .iter()
            .rev()
            .find(|&&at| class.words.is_name(at) & (class.words.depth(at) == base))
            .copied()
    }

    fn first_name(class: &ClassFile, kept: &[usize]) -> Option<usize> {
        let first = kept.first().map_or("", |&at| class.words.text(at));
        let grouped = Self::GROUPED.contains(&first);
        kept.iter()
            .find(|&&at| class.words.is_name(at) & !Self::MODIFIERS.contains(&class.words.text(at)))
            .copied()
            .filter(|_| !grouped)
    }
}
