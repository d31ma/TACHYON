#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;
use super::name_picker::NamePicker;
use std::ops::Range;

pub(super) struct BracedFields;

impl BracedFields {
    const LANGUAGES: [&'static str; 8] = ["js", "ts", "java", "cs", "dart", "kt", "swift", "php"];
    const CONTINUES: [&'static str; 12] =
        ["=", ",", "+", "-", "*", "/", ".", "?", ":", "&", "|", "("];
    const NOT_FIELDS: [&'static str; 23] = [
        "class",
        "interface",
        "enum",
        "record",
        "struct",
        "companion",
        "init",
        "constructor",
        "deinit",
        "get",
        "set",
        "case",
        "return",
        "typealias",
        "subscript",
        "operator",
        "extension",
        "protocol",
        "fun",
        "func",
        "function",
        "use",
        "}",
    ];
    const OPEN: &'static str = "(";
    const END: &'static str = ";";

    pub(super) fn read(class: &ClassFile) -> Vec<usize> {
        let wanted = Self::LANGUAGES.contains(&class.extension);
        let covered = Self::method_cover(class);
        Self::bodies(class)
            .into_iter()
            .filter(|_| wanted)
            .flat_map(|body| Self::segments(class, body))
            .filter(|segment| !covered[segment.clone()].contains(&true))
            .filter_map(|segment| Self::segment_name(class, segment))
            .collect()
    }

    fn bodies(class: &ClassFile) -> Vec<Range<usize>> {
        (0..class.words.len())
            .filter(|&at| Self::is_container(class, at))
            .map(|at| {
                class.words.inside_of(class.words.find_at_depth(
                    at + 1,
                    class.words.depth(at),
                    &["{"],
                ))
            })
            .filter(|body| !body.is_empty())
            .collect()
    }

    fn is_container(class: &ClassFile, at: usize) -> bool {
        class.spelling.is_container(class.words.text(at)) & !class.words.after_member_access(at)
    }

    fn segments(class: &ClassFile, body: Range<usize>) -> Vec<Range<usize>> {
        let depth = class.words.depth(body.start);
        let mut cuts: Vec<usize> = body
            .clone()
            .filter(|&at| (class.words.depth(at) == depth) & Self::starts_segment(class, at))
            .collect();
        cuts.push(body.end);
        let starts = std::iter::once(body.start).chain(cuts.iter().copied());
        starts
            .zip(cuts.iter().copied())
            .map(|(start, end)| Self::trimmed(class, start..end))
            .collect()
    }

    fn starts_segment(class: &ClassFile, at: usize) -> bool {
        let after_end = class.words.text(at.wrapping_sub(1)) == Self::END;
        let continues = Self::CONTINUES.contains(&class.words.text(at.wrapping_sub(1)));
        let new_line = class.words.line_start(at) & !continues;
        after_end | new_line
    }

    fn trimmed(class: &ClassFile, segment: Range<usize>) -> Range<usize> {
        let ends = class.words.text(segment.end.wrapping_sub(1)) == Self::END;
        segment.start..segment.end - usize::from(ends & (segment.end > segment.start))
    }

    fn method_cover(class: &ClassFile) -> Vec<bool> {
        let count = class.words.len();
        let mut covered = vec![false; count];
        for method in &class.methods {
            let start = class.index.line_begin(method.head);
            let end = (method.body.end + 1).max(method.head + 1).min(count);
            covered[start.min(end)..end].fill(true);
        }
        covered
    }

    fn segment_name(class: &ClassFile, segment: Range<usize>) -> Option<usize> {
        let kept = NamePicker::without_markers(class, segment);
        let first = NamePicker::first_plain_word(class, &kept);
        let calls = Self::calls_before_value(class, &kept);
        let skipped = Self::NOT_FIELDS.contains(&first) | calls;
        NamePicker::field_name(class, &kept).filter(|_| !skipped)
    }

    fn calls_before_value(class: &ClassFile, kept: &[usize]) -> bool {
        let base = kept.first().map_or(0, |&at| class.words.depth(at));
        let stop = NamePicker::equals_at(class, kept);
        kept[..stop]
            .iter()
            .any(|&at| (class.words.text(at) == Self::OPEN) & (class.words.depth(at) == base))
    }
}
