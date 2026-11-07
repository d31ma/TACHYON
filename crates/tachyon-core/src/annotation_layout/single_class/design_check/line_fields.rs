#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;

pub(super) struct LineFields;

impl LineFields {
    const LANGUAGES: [&'static str; 2] = ["py", "rb"];
    const EQUALS: &'static str = "=";
    const PYTHON_TYPE: &'static str = ":";
    const PYTHON: &'static str = "py";
    const ACCESSORS: [&'static str; 3] = ["attr_reader", "attr_accessor", "attr_writer"];
    const KEYWORDS: [&'static str; 6] = ["else", "try", "finally", "pass", "return", "except"];
    const SYMBOL: &'static str = ":";

    pub(super) fn read(class: &ClassFile) -> Vec<usize> {
        let wanted = Self::LANGUAGES.contains(&class.extension);
        let assigned =
            (0..class.words.len()).filter(|&at| wanted & Self::is_assigned_field(class, at));
        let accessed =
            (0..class.words.len()).filter(|&at| wanted & Self::is_accessor_field(class, at));
        assigned.chain(accessed).collect()
    }

    fn is_assigned_field(class: &ClassFile, at: usize) -> bool {
        let starts = class.words.line_start(at)
            & (class.words.column(at) > 0)
            & (class.words.depth(at) == 0);
        let next = class.words.text(at + 1);
        let typed = (next == Self::PYTHON_TYPE) & (class.extension == Self::PYTHON);
        let equals = (next == Self::EQUALS) & !class.words.pair(at + 1).starts_with("==");
        let assigns = typed | equals;
        let plain = !Self::KEYWORDS.contains(&class.words.text(at));
        starts & assigns & plain & class.words.is_name(at) & !class.index.in_method(at)
    }

    fn is_accessor_field(class: &ClassFile, at: usize) -> bool {
        let line = class.index.line_begin(at);
        let accessor = Self::ACCESSORS.contains(&class.words.text(line));
        let symbol = class.words.text(at.wrapping_sub(1)) == Self::SYMBOL;
        accessor & symbol & class.words.is_name(at) & (line < at)
    }
}
