#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;
use std::ops::Range;

pub(super) struct ConditionSpans {
    covered: Vec<bool>,
}

impl ConditionSpans {
    const CONDITIONS: [&'static str; 7] =
        ["if", "elif", "elsif", "unless", "while", "until", "guard"];
    const ENDS: [&'static str; 2] = ["{", ":"];
    const COMPARED: [&'static str; 2] = ["<", ">"];
    const COMPARED_PAIRS: [&'static str; 4] = ["==", "!=", "<=", ">="];
    const NOT_COMPARED: [&'static str; 3] = ["->", "=>", ".<"];
    const OPEN: &'static str = "(";
    const MINUS: &'static str = "-";

    pub(super) fn read(class: &ClassFile) -> Self {
        let spans: Vec<Range<usize>> = (0..class.words.len())
            .filter(|&at| {
                Self::CONDITIONS.contains(&class.words.text(at))
                    & !class.words.after_member_access(at)
            })
            .map(|at| Self::span(class, at))
            .collect();
        Self {
            covered: Self::cover(class.words.len(), &spans),
        }
    }

    fn cover(count: usize, spans: &[Range<usize>]) -> Vec<bool> {
        let mut covered = vec![false; count];
        for span in spans {
            let end = span.end.min(count);
            covered[span.start.min(end)..end].fill(true);
        }
        covered
    }

    pub(super) fn holds(&self, class: &ClassFile, at: usize) -> bool {
        let spanned = self.covered.get(at).copied().unwrap_or(false);
        spanned | Self::compared(class, at)
    }

    fn span(class: &ClassFile, at: usize) -> Range<usize> {
        if class.words.text(at + 1) == Self::OPEN {
            return class.words.inside(at + 1);
        }
        let depth = class.words.depth(at);
        let end = (at + 1..class.words.len())
            .find(|&index| {
                let ends = Self::ENDS.contains(&class.words.text(index))
                    & (class.words.depth(index) == depth);
                ends | class.words.line_start(index)
            })
            .unwrap_or(class.words.len());
        at + 1..end
    }

    fn compared(class: &ClassFile, at: usize) -> bool {
        let words = &class.words;
        let signed = usize::from(words.text(at.wrapping_sub(1)) == Self::MINUS);
        let before = at.wrapping_sub(1 + signed);
        let before_single = Self::COMPARED.contains(&words.text(before))
            & !Self::NOT_COMPARED.contains(&words.pair(before.wrapping_sub(1)));
        let before_pair = Self::COMPARED_PAIRS.contains(&words.pair(before.wrapping_sub(1)));
        let after_single = Self::COMPARED.contains(&words.text(at + 1))
            & !Self::NOT_COMPARED.contains(&words.pair(at + 1));
        let after_pair = Self::COMPARED_PAIRS.contains(&words.pair(at + 1));
        before_single | before_pair | after_single | after_pair
    }
}
