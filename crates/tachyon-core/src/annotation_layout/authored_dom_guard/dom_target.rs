use super::dom_expression::DomExpression;
use crate::annotation_layout::single_class::Token;

pub(super) struct DomTarget<'a, 's> {
    tokens: &'a [Token<'s>],
}

impl<'a, 's> DomTarget<'a, 's> {
    pub(super) fn new(tokens: &'a [Token<'s>]) -> Self {
        Self { tokens }
    }

    pub(super) fn path(&self, end: usize) -> Option<String> {
        self.segments(end).map(|segments| segments.join("."))
    }

    fn segments(&self, end: usize) -> Option<Vec<&'s str>> {
        let mut segments = Vec::new();
        let mut next = Some(end);
        while let Some(segment_end) = next {
            let (name, receiver_end) = self.segment(segment_end)?;
            segments.push(name);
            next = receiver_end;
        }
        segments.reverse();
        Some(segments)
    }

    fn segment(&self, end: usize) -> Option<(&'s str, Option<usize>)> {
        self.bracket_segment(end).or_else(|| self.name_segment(end))
    }

    fn name_segment(&self, end: usize) -> Option<(&'s str, Option<usize>)> {
        let name = self.name(end)?;
        if !self.dotted(end) {
            return Some((name, None));
        }
        let receiver_end = end
            .checked_sub(2)
            .filter(|receiver_end| self.ends_segment(*receiver_end))?;
        Some((name, Some(receiver_end)))
    }

    fn bracket_segment(&self, end: usize) -> Option<(&'s str, Option<usize>)> {
        let close = (self.text(end) == "]").then_some(end)?;
        let open = close
            .checked_sub(2)
            .filter(|open| self.text(*open) == "[")?;
        let name = self
            .tokens
            .get(open + 1)
            .filter(|token| token.literal)
            .and_then(|token| DomExpression::literal(token.text))?;
        let receiver_end = open
            .checked_sub(1)
            .filter(|receiver_end| self.ends_segment(*receiver_end))?;
        Some((name, Some(receiver_end)))
    }

    fn dotted(&self, end: usize) -> bool {
        end.checked_sub(1)
            .is_some_and(|before| self.text(before) == ".")
    }

    fn ends_segment(&self, at: usize) -> bool {
        [self.text(at) == "]", self.name(at).is_some()].contains(&true)
    }

    fn name(&self, at: usize) -> Option<&'s str> {
        self.tokens
            .get(at)
            .filter(|token| !token.literal)
            .map(|token| token.text)
            .filter(|text| DomExpression::identifier(text))
    }

    fn text(&self, at: usize) -> &'s str {
        self.tokens.get(at).map_or("", |token| token.text)
    }
}
