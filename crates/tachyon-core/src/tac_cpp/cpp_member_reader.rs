use super::{CppAttribute, CppAttributes, CppMember, CppMemberKind, CppParameter, CppTokens};
use std::ops::Range;

pub(crate) struct CppMemberReader<'a> {
    tokens: &'a CppTokens,
    source: &'a str,
    class: &'a str,
    public: bool,
}

struct CppShape {
    parameters: Option<usize>,
    cut: Option<usize>,
    initialized: bool,
    commas: usize,
    operator: bool,
}

impl<'a> CppMemberReader<'a> {
    const ACCESS: [&'static str; 3] = ["public", "private", "protected"];
    const SPECIFIERS: [&'static str; 9] = [
        "static",
        "virtual",
        "inline",
        "constexpr",
        "explicit",
        "mutable",
        "consteval",
        "constinit",
        "thread_local",
    ];
    const NESTED: [&'static str; 4] = ["class", "struct", "enum", "union"];
    const OTHER: [&'static str; 5] = ["using", "friend", "static_assert", "typedef", "template"];
    const CUTS: [&'static str; 4] = ["=", "{", "[", ":"];

    pub(crate) fn read(
        tokens: &'a CppTokens,
        source: &'a str,
        class: &'a str,
        public: bool,
        body: Range<usize>,
    ) -> Vec<CppMember> {
        let mut reader = Self {
            tokens,
            source,
            class,
            public,
        };
        reader.members(body)
    }

    fn members(&mut self, body: Range<usize>) -> Vec<CppMember> {
        let mut members = Vec::new();
        let mut at = body.start;
        while at < body.end {
            at = self.next_member(at, body.end, &mut members);
        }
        members
    }

    fn next_member(&mut self, at: usize, limit: usize, members: &mut Vec<CppMember>) -> usize {
        if self.tokens.is(at, ";") {
            return at + 1;
        }
        let access = Self::ACCESS.contains(&self.tokens.text(at)) && self.tokens.is(at + 1, ":");
        if access {
            self.public = self.tokens.is(at, "public");
            return at + 2;
        }
        let (markers, start) = CppAttributes::read(self.tokens, self.source, at);
        let (end, declared) = self.member_end(start, limit);
        members.push(self.member(markers, start..declared));
        end
    }

    fn member_end(&self, start: usize, limit: usize) -> (usize, usize) {
        let mut at = start;
        let mut parameters = false;
        let mut equals = false;
        let mut initializers = false;
        let mut angles = 0_usize;
        while at < limit {
            match self.tokens.text(at) {
                ";" => return (at + 1, at),
                "=" if angles == 0 => equals = true,
                "<" => angles += 1,
                ">" => angles = angles.saturating_sub(1),
                ":" if parameters && !equals => initializers = true,
                "(" if angles == 0 && !equals && !parameters => parameters = true,
                "{" if self.starts_body(at, parameters && !equals, initializers) => {
                    return (self.after_body(at), at);
                }
                _ => {}
            }
            at = self
                .tokens
                .partner(at)
                .filter(|partner| *partner > at)
                .unwrap_or(at)
                + 1;
        }
        (limit, limit)
    }

    fn starts_body(&self, at: usize, after_parameters: bool, initializers: bool) -> bool {
        let previous = self.tokens.text(at.saturating_sub(1));
        let member_initializer = initializers && (previous == ">" || self.is_word_token(at - 1));
        after_parameters && !member_initializer
    }

    fn is_word_token(&self, index: usize) -> bool {
        self.tokens
            .list
            .get(index)
            .is_some_and(super::CppToken::is_word)
    }

    fn after_body(&self, open: usize) -> usize {
        let close = self.tokens.partner(open).unwrap_or(open);
        close + 1 + usize::from(self.tokens.is(close + 1, ";"))
    }

    fn shape(&self, declaration: &Range<usize>) -> CppShape {
        let mut shape = CppShape {
            parameters: None,
            cut: None,
            initialized: false,
            commas: 0,
            operator: false,
        };
        let mut angles = 0_usize;
        let mut at = declaration.start;
        while at < declaration.end {
            let text = self.tokens.text(at);
            shape.operator |= text == "operator";
            match text {
                "<" => angles += 1,
                ">" => angles = angles.saturating_sub(1),
                "," if angles == 0 => shape.commas += 1,
                "(" if angles == 0 && shape.cut.is_none() && shape.parameters.is_none() => {
                    shape.parameters = Some(at);
                }
                _ => {}
            }
            if angles == 0
                && shape.cut.is_none()
                && shape.parameters.is_none()
                && Self::CUTS.contains(&text)
            {
                shape.cut = Some(at);
                shape.initialized = matches!(text, "=" | "{");
            }
            at = self
                .tokens
                .partner(at)
                .filter(|partner| *partner > at)
                .unwrap_or(at)
                + 1;
        }
        shape
    }

    fn member(&self, markers: Vec<CppAttribute>, declaration: Range<usize>) -> CppMember {
        let shape = self.shape(&declaration);
        let first = self.tokens.text(declaration.start);
        let mut member = CppMember {
            kind: CppMemberKind::Other,
            name: String::new(),
            name_at: self.tokens.start(declaration.start).min(self.source.len()),
            type_text: String::new(),
            parameters: Vec::new(),
            public: self.public,
            is_static: false,
            has_initializer: shape.initialized,
            declarators: shape.commas + 1,
            markers,
            terminator: self.tokens.start(declaration.end).min(self.source.len()),
        };
        if Self::OTHER.contains(&first) || declaration.is_empty() {
            return member;
        }
        let specifiers = self.specifiers(declaration.clone());
        member.is_static =
            (declaration.start..specifiers).any(|index| self.tokens.is(index, "static"));
        if Self::NESTED.contains(&self.tokens.text(specifiers)) {
            member.kind = CppMemberKind::NestedType;
            self.tokens
                .text(specifiers + 1)
                .clone_into(&mut member.name);
            return member;
        }
        match (shape.operator, shape.parameters) {
            (true, _) => member.kind = CppMemberKind::Operator,
            (false, Some(open)) => self.function(&mut member, specifiers, open, declaration.end),
            (false, None) => self.field(
                &mut member,
                specifiers,
                shape.cut.unwrap_or(declaration.end),
            ),
        }
        member
    }

    fn specifiers(&self, declaration: Range<usize>) -> usize {
        (declaration.start..declaration.end)
            .find(|index| !Self::SPECIFIERS.contains(&self.tokens.text(*index)))
            .unwrap_or(declaration.end)
    }

    fn text_between(&self, first: usize, after: usize) -> String {
        if first >= after {
            return String::new();
        }
        let start = self.tokens.start(first).min(self.source.len());
        let end = self.tokens.end(after - 1).min(self.source.len());
        self.source[start..end]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn function(&self, member: &mut CppMember, specifiers: usize, open: usize, end: usize) {
        let name_index = open.saturating_sub(1);
        self.tokens.text(name_index).clone_into(&mut member.name);
        member.name_at = self.tokens.start(name_index);
        let destructor = self.tokens.is(name_index.saturating_sub(1), "~");
        member.kind = match (destructor, member.name == self.class) {
            (true, _) => CppMemberKind::Destructor,
            (false, true) => CppMemberKind::Constructor,
            (false, false) => CppMemberKind::Method,
        };
        member.type_text = self.text_between(specifiers, name_index);
        let close = self.tokens.partner(open).unwrap_or(end);
        member.type_text = self
            .trailing_return(close, end)
            .unwrap_or(member.type_text.clone());
        member.parameters = self.parameters(open + 1, close);
    }

    fn trailing_return(&self, close: usize, end: usize) -> Option<String> {
        let arrow = (close..end).find(|index| self.tokens.is(*index, "->"))?;
        let stop = (arrow..end)
            .find(|index| matches!(self.tokens.text(*index), "{" | "=" | "override" | "final"))
            .unwrap_or(end);
        Some(self.text_between(arrow + 1, stop))
    }

    fn field(&self, member: &mut CppMember, specifiers: usize, cut: usize) {
        let name_index = (specifiers..cut)
            .rev()
            .find(|index| self.is_word_token(*index));
        let Some(name_index) = name_index else {
            return;
        };
        member.kind = CppMemberKind::Field;
        self.tokens.text(name_index).clone_into(&mut member.name);
        member.name_at = self.tokens.start(name_index);
        member.type_text = self.text_between(specifiers, name_index);
    }

    fn parameters(&self, start: usize, end: usize) -> Vec<CppParameter> {
        let mut parameters = Vec::new();
        let mut first = start;
        let mut at = start;
        let mut angles = 0_usize;
        while at < end {
            match self.tokens.text(at) {
                "<" => angles += 1,
                ">" => angles = angles.saturating_sub(1),
                "," if angles == 0 => {
                    parameters.extend(self.parameter(first, at));
                    first = at + 1;
                }
                _ => {}
            }
            at = self
                .tokens
                .partner(at)
                .filter(|partner| *partner > at)
                .unwrap_or(at)
                + 1;
        }
        parameters.extend(self.parameter(first, end));
        parameters
    }

    fn parameter(&self, start: usize, end: usize) -> Option<CppParameter> {
        let (markers, first) = CppAttributes::read(self.tokens, self.source, start);
        let cut = (first..end)
            .find(|index| self.tokens.is(*index, "="))
            .unwrap_or(end);
        if first >= cut || self.tokens.is(first, "void") && cut == first + 1 {
            return None;
        }
        let last = cut - 1;
        let named = cut - first > 1 && self.is_word_token(last);
        let type_end = if named { last } else { cut };
        Some(CppParameter {
            name: if named {
                self.tokens.text(last).to_owned()
            } else {
                String::new()
            },
            type_text: self.text_between(first, type_end),
            markers,
        })
    }
}
