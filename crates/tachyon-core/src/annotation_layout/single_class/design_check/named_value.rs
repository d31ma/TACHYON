#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;
use std::collections::BTreeSet;

pub(super) struct NamedValue<'c, 'a> {
    class: &'c ClassFile<'a>,
    returned: BTreeSet<usize>,
}

impl<'c, 'a> NamedValue<'c, 'a> {
    const OPERATORS: [&'static str; 11] = ["+", "-", "*", "/", "%", "<", ">", "!", "&", "|", "?"];
    const VALUE_BEFORE: [&'static str; 8] = ["=", ",", "(", "[", "{", ":", "?", "return"];
    const GO_TYPE_MARKS: [&'static str; 3] = ["[", "]", "*"];
    const GO: &'static str = "go";
    const COMPARED_BEFORE: [&'static str; 4] = ["=", "!", "<", ">"];
    const COMPARED_AFTER: [&'static str; 2] = ["=", ">"];
    const STOP_WORDS: [&'static str; 8] =
        ["return", "if", "else", "case", "when", "yield", "in", "not"];
    const NAMED_BRACES: [&'static str; 2] = ["go", "rs"];
    const RETURN_NOISE: [&'static str; 7] = ["return", ";", "{", "}", "-", "(", ")"];
    const EQUALS: &'static str = "=";
    const DECLARATION_REACH: usize = 16;
    const CALL: &'static str = "(";
    const PAIR_ARROW: &'static str = "=>";
    const CONSTANT_WORDS: [&'static str; 3] = ["const", "static", "readonly"];
    const LOCAL_CONST: [&'static str; 2] = ["js", "ts"];
    const BRACE: &'static str = "{";
    const DOT: &'static str = ".";
    const COLON: &'static str = ":";
    const NEW: &'static str = "new";

    pub(super) fn new(class: &'c ClassFile<'a>) -> Self {
        Self {
            class,
            returned: Self::returned(class),
        }
    }

    pub(super) fn is_named(&self, at: usize) -> bool {
        self.returned.contains(&at) | self.assigned(at)
    }

    fn assigned(&self, at: usize) -> bool {
        let mut level = Some(at);
        let mut through_call = false;
        let mut verdict = None;
        while let (Some(head), None) = (level, verdict) {
            through_call |= self.class.words.text(head) == Self::CALL;
            verdict = self.verdict_at(head, through_call);
            level = self.passable_parent(head);
        }
        verdict.unwrap_or(false)
    }

    fn verdict_at(&self, head: usize, through_call: bool) -> Option<bool> {
        let before = self.path_start(head);
        let text = self.class.words.text(before);
        if text == Self::EQUALS {
            let constant = !through_call | self.declares_constant(before);
            return Some(self.is_assignment(before) & self.ends_cleanly(head) & constant);
        }
        let pair = self.class.words.pair(before.wrapping_sub(1)) == Self::PAIR_ARROW;
        (Self::OPERATORS.contains(&text) & !pair).then_some(false)
    }

    fn declares_constant(&self, equals: usize) -> bool {
        let words = &self.class.words;
        let local_const = Self::LOCAL_CONST.contains(&self.class.extension);
        let start = self
            .class
            .index
            .line_begin(equals)
            .max(equals.saturating_sub(Self::DECLARATION_REACH));
        (start..equals).any(|at| {
            let text = words.text(at);
            let keyword = Self::CONSTANT_WORDS.contains(&text) & !(local_const & (text == "const"));
            keyword | Self::is_upper_name(text)
        })
    }

    fn is_upper_name(text: &str) -> bool {
        let letters = text.chars().any(|character| character.is_ascii_uppercase());
        let upper = text.chars().all(|character| {
            character.is_ascii_uppercase() | character.is_ascii_digit() | (character == '_')
        });
        letters & upper & (text.len() > 1)
    }

    fn path_start(&self, head: usize) -> usize {
        let mut at = head.wrapping_sub(1);
        while self.on_path(at, head) {
            at = at.wrapping_sub(1);
        }
        at
    }

    fn on_path(&self, at: usize, head: usize) -> bool {
        let words = &self.class.words;
        let text = words.text(at);
        let name = words.is_name(at) & !Self::STOP_WORDS.contains(&text);
        let colons = (text == Self::COLON)
            & ((words.text(at.wrapping_sub(1)) == Self::COLON)
                | (words.text(at.wrapping_add(1)) == Self::COLON));
        let unary = (text == "-") & (at.wrapping_add(1) == head);
        let go_type = (self.class.extension == Self::GO) & Self::GO_TYPE_MARKS.contains(&text);
        name | (text == Self::DOT) | (text == Self::NEW) | colons | unary | go_type
    }

    fn is_assignment(&self, equals: usize) -> bool {
        let words = &self.class.words;
        let before = Self::COMPARED_BEFORE.contains(&words.text(equals.wrapping_sub(1)))
            & words.touches_previous(equals);
        let after = Self::COMPARED_AFTER.contains(&words.text(equals + 1))
            & words.touches_previous(equals + 1);
        !(before | after)
    }

    fn ends_cleanly(&self, head: usize) -> bool {
        let end = self.class.words.after_group(head).max(head + 1);
        !Self::OPERATORS.contains(&self.class.words.text(end))
    }

    fn passable_parent(&self, head: usize) -> Option<usize> {
        let parent = self.class.index.parent(head)?;
        self.passable(parent).then_some(parent)
    }

    fn passable(&self, open: usize) -> bool {
        let words = &self.class.words;
        let brace = words.text(open) == Self::BRACE;
        let before = words.text(open.wrapping_sub(1));
        let value = Self::VALUE_BEFORE.contains(&before);
        let named = words.is_name(open.wrapping_sub(1))
            & Self::NAMED_BRACES.contains(&self.class.extension);
        !brace | value | named
    }

    fn returned(class: &ClassFile) -> BTreeSet<usize> {
        class
            .methods
            .iter()
            .filter_map(|method| Self::only_value(class, method.body.clone()))
            .collect()
    }

    fn only_value(class: &ClassFile, body: std::ops::Range<usize>) -> Option<usize> {
        let kept: Vec<usize> = body
            .filter(|&at| !Self::RETURN_NOISE.contains(&class.words.text(at)))
            .collect();
        (kept.len() == 1).then(|| kept[0])
    }
}
