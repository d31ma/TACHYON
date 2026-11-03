use super::super::cohesion::Method;
use super::class_file::ClassFile;
use std::ops::Range;

pub(super) struct Signature {
    pub(super) name_at: usize,
    pub(super) parameters: Range<usize>,
}

impl Signature {
    const OPEN: &'static str = "(";
    const NAME_REACH: usize = 16;
    const OPEN_REACH: usize = 64;

    pub(super) fn of(class: &ClassFile, method: &Method) -> Self {
        let name_end = (method.head + Self::NAME_REACH).min(class.words.len());
        let name_at = (method.head..name_end)
            .find(|&at| class.words.text(at) == method.name)
            .unwrap_or(method.head);
        let open_end = class
            .index
            .line_end(name_at)
            .min(name_at + Self::OPEN_REACH);
        let open = (name_at + 1..open_end).find(|&at| class.words.text(at) == Self::OPEN);
        Self {
            name_at,
            parameters: class.words.inside_of(open),
        }
    }
}
