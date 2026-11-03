#![allow(clippy::needless_bitwise_bool)]

use super::super::cohesion::Method;
use super::class_file::ClassFile;

pub(super) struct MethodFilter;

impl MethodFilter {
    const OVERRIDES: [&'static str; 2] = ["override", "Override"];
    const CONSTRUCTORS: [&'static str; 5] =
        ["constructor", "init", "initialize", "__construct", "new"];
    const DUNDER: &'static str = "__";
    const IMPLEMENTATION: &'static str = "impl";
    const FOR: &'static str = "for";
    const OWNERS: [&'static str; 6] = ["class", "struct", "record", "object", "extension", "actor"];
    const BASE_WORDS: [&'static str; 3] = ["extends", "implements", "with"];
    const LINE_OWNED: [&'static str; 2] = ["py", "rb"];
    const LINE_BASE: [&'static str; 2] = ["(", "<"];
    const COLON: &'static str = ":";
    const HEADER_REACH: usize = 64;

    pub(super) fn names_are_chosen(class: &ClassFile, method: &Method) -> bool {
        let special = method.name.starts_with(Self::DUNDER);
        let inherited =
            Self::overrides(class, method) | Self::in_trait_implementation(class, method);
        !(special | inherited | Self::in_derived_class(class, method))
    }

    pub(super) fn is_constructor(class: &ClassFile, method: &Method) -> bool {
        let named =
            Self::CONSTRUCTORS.contains(&method.name.as_str()) | (method.name == class.name);
        named | method.name.starts_with(Self::DUNDER)
    }

    fn overrides(class: &ClassFile, method: &Method) -> bool {
        let line = class.index.line_begin(method.head);
        let previous = class.index.line_begin(line.saturating_sub(1));
        class
            .words
            .has_word(previous..method.head, &Self::OVERRIDES)
    }

    fn in_trait_implementation(class: &ClassFile, method: &Method) -> bool {
        let owner = Self::header_owner(class, method, &[Self::IMPLEMENTATION]);
        owner.is_some_and(|at| Self::names_a_trait(class, at))
    }

    fn names_a_trait(class: &ClassFile, at: usize) -> bool {
        let open = Self::header_end(class, at);
        class.words.has_word(at..open, &[Self::FOR])
    }

    fn header_end(class: &ClassFile, at: usize) -> usize {
        let words = &class.words;
        let line_end = class.index.line_end(at);
        (at + 1..line_end)
            .find(|&index| (words.text(index) == "{") & (words.depth(index) == words.depth(at)))
            .unwrap_or(line_end)
    }

    fn in_derived_class(class: &ClassFile, method: &Method) -> bool {
        let line_owned = Self::LINE_OWNED.contains(&class.extension);
        let braced = Self::header_owner(class, method, &Self::OWNERS).filter(|_| !line_owned);
        let lined = class.index.last_class(method.head).filter(|_| line_owned);
        braced
            .or(lined)
            .is_some_and(|at| Self::names_a_base(class, at, line_owned))
    }

    fn header_owner(class: &ClassFile, method: &Method, keywords: &[&str]) -> Option<usize> {
        let open = class.index.parent(method.head)?;
        let depth = class.words.depth(open);
        let start = open.saturating_sub(Self::HEADER_REACH);
        (start..open).rev().find(|&at| {
            let keyword = keywords.contains(&class.words.text(at));
            keyword & (class.words.depth(at) == depth) & !class.words.after_member_access(at)
        })
    }

    fn names_a_base(class: &ClassFile, at: usize, line_owned: bool) -> bool {
        let words = &class.words;
        let header = at + 1..Self::header_end(class, at);
        let keyword = words.has_word(header.clone(), &Self::BASE_WORDS);
        let colon = header.clone().any(|index| {
            let level = words.depth(index) == words.depth(at);
            let single = (words.pair(index) != "::") & (words.pair(index.wrapping_sub(1)) != "::");
            (words.text(index) == Self::COLON) & level & single
        });
        let line_base = line_owned & words.has_word(header, &Self::LINE_BASE);
        keyword | (colon & !line_owned) | line_base
    }
}
