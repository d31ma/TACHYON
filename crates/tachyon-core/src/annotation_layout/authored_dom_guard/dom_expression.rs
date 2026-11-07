use super::dom_bindings::DomBindings;
use super::dom_kind::DomKind;
use super::dom_object::DomObject;
use crate::annotation_layout::single_class::Token;

pub(super) struct DomExpression<'a, 's> {
    tokens: &'a [Token<'s>],
    bindings: &'a DomBindings,
}

impl<'a, 's> DomExpression<'a, 's> {
    const MAX_CHAIN: usize = 256;
    const QUOTES: [char; 2] = ['\'', '"'];

    pub(super) fn new(tokens: &'a [Token<'s>], bindings: &'a DomBindings) -> Self {
        Self { tokens, bindings }
    }

    pub(super) fn kind(&self, start: usize, end: usize) -> DomKind {
        self.expression(start, end)
    }

    pub(super) fn receiver(&self, at: usize) -> DomKind {
        let start = self.receiver_start(at);
        self.expression(start, at)
    }

    pub(super) fn property(&self, at: usize) -> Option<(&'s str, usize)> {
        self.dot_property(at).or_else(|| self.bracket_property(at))
    }

    pub(super) fn member_reference(&self, at: usize) -> DomKind {
        let start = self.receiver_start(at);
        self.property(at)
            .map_or(DomKind::Unknown, |(_, end)| self.expression(start, end))
    }

    pub(super) fn arguments_create_nodes(&self, start: usize) -> bool {
        self.arguments(start)
            .into_iter()
            .any(|(start, end)| self.expression(start, end) == DomKind::CreatedNode)
    }

    pub(super) fn literal_argument(&self, start: usize) -> Option<&'s str> {
        self.tokens
            .get(start + 1)
            .filter(|token| token.literal)
            .and_then(|token| Self::literal(token.text))
    }

    pub(super) fn identifier(name: &str) -> bool {
        name.as_bytes().first().is_some_and(|byte| {
            [byte.is_ascii_alphabetic(), *byte == b'_', *byte == b'$'].contains(&true)
        })
    }

    fn text(&self, at: usize) -> &str {
        self.tokens.get(at).map_or("", |token| token.text)
    }

    fn dot_property(&self, at: usize) -> Option<(&'s str, usize)> {
        (self.text(at) == ".")
            .then(|| self.tokens.get(at + 1))
            .flatten()
            .filter(|token| !token.literal)
            .map(|token| (token.text, at + 2))
    }

    fn bracket_property(&self, at: usize) -> Option<(&'s str, usize)> {
        [self.text(at) == "[", self.text(at + 2) == "]"]
            .contains(&false)
            .then_some(())
            .map_or_else(
                || {
                    self.tokens
                        .get(at + 1)
                        .filter(|token| token.literal)
                        .and_then(|token| Self::literal(token.text))
                        .map(|name| (name, at + 3))
                },
                |()| None,
            )
    }

    pub(super) fn literal(text: &str) -> Option<&str> {
        let quote = text.chars().next()?;
        Self::QUOTES.contains(&quote).then_some(())?;
        let value = text.strip_prefix(quote)?.strip_suffix(quote)?;
        (!value.contains('\\')).then_some(value)
    }

    fn receiver_start(&self, at: usize) -> usize {
        let mut start = at;
        let mut depth = 0_usize;
        let boundary = (at.saturating_sub(Self::MAX_CHAIN)..at)
            .rev()
            .find_map(|before| {
                let text = self.text(before);
                let boundary = Self::ends_receiver(text, depth).then_some(start);
                depth = Self::reverse_depth(depth, text);
                start = before;
                boundary
            });
        boundary.unwrap_or(start)
    }

    fn reverse_depth(depth: usize, text: &str) -> usize {
        let opens = usize::from([")", "]"].contains(&text));
        let closes = usize::from(["(", "["].contains(&text));
        (depth + opens).saturating_sub(closes)
    }

    fn ends_receiver(text: &str, depth: usize) -> bool {
        [
            depth == 0,
            [";", "=", ",", "{", "}", "return", "=>", ":", "("].contains(&text),
        ]
        .into_iter()
        .all(|holds| holds)
    }

    fn expression(&self, start: usize, end: usize) -> DomKind {
        let start = self.without_prefix(start, end);
        let mut at = self.first_step(start);
        let mut kind = self.initial(start);
        let mut name = self.text(start).to_owned();
        while at < end.min(start.saturating_add(Self::MAX_CHAIN)) {
            let (next_kind, next_at, next_name) = self.step(kind, at, &name);
            kind = next_kind;
            at = next_at;
            name = next_name;
        }
        kind
    }

    fn without_prefix(&self, start: usize, end: usize) -> usize {
        if [
            start + 1 < end,
            ["new", "await", "("].contains(&self.text(start)),
        ]
        .contains(&false)
        {
            return start;
        }
        start + 1
    }

    fn first_step(&self, start: usize) -> usize {
        if self.text(start) == "{" {
            return self.group_end(start);
        }
        start + 1
    }

    fn initial(&self, start: usize) -> DomKind {
        if self.text(start) == "{" {
            return DomObject::new(self.tokens, self.bindings).kind(start, self.group_end(start));
        }
        self.tokens
            .get(start)
            .filter(|token| token.literal)
            .map_or_else(|| self.bindings.kind(self.text(start)), |_| DomKind::Data)
    }

    fn step(&self, receiver: DomKind, at: usize, name: &str) -> (DomKind, usize, String) {
        self.property(at).map_or_else(
            || self.nonmember(receiver, at, name),
            |(member, after)| self.member(receiver, name, member, after),
        )
    }

    fn member(
        &self,
        receiver: DomKind,
        name: &str,
        member: &str,
        after: usize,
    ) -> (DomKind, usize, String) {
        let name = format!("{name}.{member}");
        let bound = self.bindings.kind(&name);
        let inferred = self.member_kind(receiver, member, after);
        let kind = Self::bound_kind(bound, inferred);
        let end = self.member_end(after);
        (kind, end, name)
    }

    fn bound_kind(bound: DomKind, inferred: DomKind) -> DomKind {
        if bound != DomKind::Unknown {
            return bound;
        }
        inferred
    }

    fn member_end(&self, after: usize) -> usize {
        if self.text(after) == "(" {
            return self.group_end(after);
        }
        after
    }

    fn member_kind(&self, receiver: DomKind, member: &str, after: usize) -> DomKind {
        if self.text(after) == "(" {
            return receiver.called(member);
        }
        receiver.member(member)
    }

    fn nonmember(&self, kind: DomKind, at: usize, name: &str) -> (DomKind, usize, String) {
        let kind = self.group_kind(kind, at).after_operator(self.text(at));
        let end = self.nonmember_end(at);
        (kind, end, name.to_owned())
    }

    fn nonmember_end(&self, at: usize) -> usize {
        if ["(", "["].contains(&self.text(at)) {
            return self.group_end(at);
        }
        at + 1
    }

    fn group_kind(&self, kind: DomKind, at: usize) -> DomKind {
        if self.text(at) == "(" {
            return kind.invoked();
        }
        kind
    }

    fn group_end(&self, start: usize) -> usize {
        let mut depth = 0_usize;
        (start..self.tokens.len())
            .find(|at| {
                depth = Self::forward_depth(depth, self.text(*at));
                depth == 0
            })
            .map_or(self.tokens.len(), |at| at + 1)
    }

    fn forward_depth(depth: usize, text: &str) -> usize {
        let opens = usize::from(["(", "[", "{"].contains(&text));
        let closes = usize::from([")", "]", "}"].contains(&text));
        (depth + opens).saturating_sub(closes)
    }

    fn record_argument(
        arguments: &mut Vec<(usize, usize)>,
        first: &mut usize,
        at: usize,
        separator: bool,
    ) {
        if separator {
            arguments.push((*first, at));
            *first = at + 1;
        }
    }

    fn arguments(&self, start: usize) -> Vec<(usize, usize)> {
        let end = self.group_end(start).saturating_sub(1);
        let mut result = Vec::new();
        let mut depth = 0_usize;
        let mut first = start + 1;
        for at in start + 1..end {
            let text = self.text(at);
            let separator = [depth == 0, text == ","].into_iter().all(|holds| holds);
            Self::record_argument(&mut result, &mut first, at, separator);
            depth = Self::forward_depth(depth, text);
        }
        result.push((first, end));
        result
    }
}
