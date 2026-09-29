use super::contract_member::ContractMember;
use super::indented_syntax::IndentedSyntax;
use crate::behavior::HttpClientSource;
use std::ops::Range;

pub(crate) struct IndentedMembers<'a> {
    code: &'a str,
    lines: Vec<Range<usize>>,
    syntax: IndentedSyntax,
}

impl<'a> IndentedMembers<'a> {
    const NAME_PUNCTUATION: [u8; 2] = *b"_.";
    const INLINE_BODY: char = ':';

    pub(crate) fn members(code: &'a str, body: Range<usize>, ruby: bool) -> Vec<ContractMember> {
        let lines = code[body.clone()]
            .split_inclusive('\n')
            .scan(body.start, |offset, line| {
                let start = *offset;
                *offset += line.len();
                Some(start..*offset)
            })
            .collect();
        let syntax = IndentedSyntax::of(ruby);
        let reader = Self {
            code,
            lines,
            syntax,
        };
        reader.read_all()
    }

    fn indent(&self, index: usize) -> usize {
        let text = &self.code[self.lines[index].clone()];
        text.len() - text.trim_start().len()
    }

    fn text(&self, index: usize) -> &str {
        self.lines
            .get(index)
            .map_or("", |line| self.code[line.clone()].trim())
    }

    fn read_all(&self) -> Vec<ContractMember> {
        (0..self.lines.len())
            .find(|index| !self.text(*index).is_empty())
            .map_or_else(Vec::new, |first| self.read_from(first))
    }

    fn read_from(&self, first: usize) -> Vec<ContractMember> {
        let member_indent = self.indent(first);
        let mut members = Vec::new();
        let mut pending = ContractMember::default();
        let mut index = first;
        while index < self.lines.len() {
            index = self.read_line(index, member_indent, &mut pending, &mut members);
        }
        members
    }

    fn is_member_line(&self, index: usize, member_indent: usize) -> bool {
        [
            self.text(index).is_empty(),
            self.indent(index) == member_indent,
        ] == [false, true]
    }

    fn read_line(
        &self,
        index: usize,
        member_indent: usize,
        pending: &mut ContractMember,
        members: &mut Vec<ContractMember>,
    ) -> usize {
        let place = (index, member_indent);
        match (
            self.is_member_line(index, member_indent),
            self.marker(self.text(index)),
        ) {
            (false, _) => index + 1,
            (true, Some(name)) => self.take_marker(index, &name, pending),
            (true, None) => self.take_member(place, pending, members),
        }
    }

    fn is_name_byte(byte: u8) -> bool {
        [
            byte.is_ascii_alphanumeric(),
            Self::NAME_PUNCTUATION.contains(&byte),
        ]
        .contains(&true)
    }

    fn marker(&self, text: &str) -> Option<String> {
        let rest = text.strip_prefix(self.syntax.marker_prefix)?;
        let length = rest
            .bytes()
            .take_while(|byte| Self::is_name_byte(*byte))
            .count();
        let name = rest[..length].rsplit('.').next().unwrap_or("").to_owned();
        (length > 0).then_some(name)
    }

    fn take_marker(&self, index: usize, name: &str, pending: &mut ContractMember) -> usize {
        pending.first_marker.get_or_insert(self.lines[index].start);
        pending.markers.push(HttpClientSource::normalized(name));
        self.after_open_group(index)
    }

    fn take_member(
        &self,
        (index, member_indent): (usize, usize),
        pending: &mut ContractMember,
        members: &mut Vec<ContractMember>,
    ) -> usize {
        let mut member = std::mem::take(pending);
        member.start = self.lines[index].start + self.indent(index);
        let next = self.read_member(index, member_indent, &mut member);
        members.push(member);
        next
    }

    fn read_member(
        &self,
        index: usize,
        member_indent: usize,
        member: &mut ContractMember,
    ) -> usize {
        if self.is_method(self.text(index)) {
            self.method(index, member_indent, member)
        } else {
            self.field(index, member)
        }
    }

    fn is_method(&self, text: &str) -> bool {
        self.syntax
            .methods
            .iter()
            .any(|prefix| text.starts_with(prefix))
    }

    fn field(&self, index: usize, member: &mut ContractMember) -> usize {
        let line = self.lines[index].clone();
        member.end = line.start + self.code[line].trim_end().len();
        index + 1
    }

    fn paren_change(&self, index: usize) -> i64 {
        let text = &self.code[self.lines[index].clone()];
        let opened = i64::try_from(text.matches('(').count()).unwrap_or(i64::MAX);
        let closed = i64::try_from(text.matches(')').count()).unwrap_or(i64::MAX);
        opened - closed
    }

    fn after_open_group(&self, index: usize) -> usize {
        (index..self.lines.len())
            .scan(0_i64, |depth, current| {
                *depth = (*depth + self.paren_change(current)).max(0);
                Some((*depth, current))
            })
            .find(|(depth, _)| *depth == 0)
            .map_or(self.lines.len(), |(_, current)| current + 1)
    }

    fn method(&self, index: usize, member_indent: usize, member: &mut ContractMember) -> usize {
        member.method = true;
        let signature_last = self.after_open_group(index) - 1;
        let line = self.lines[signature_last].clone();
        let line_text = &self.code[line.clone()];
        let inline_body = self
            .syntax
            .inline_bodies
            .then(|| line_text.rfind(Self::INLINE_BODY))
            .flatten()
            .map(|colon| line.start + colon + 1);
        member.end = inline_body.map_or(line.start + line_text.trim_end().len(), |colon| colon - 1);
        let next = (signature_last + 1..self.lines.len())
            .find(|current| !self.belongs_to_method(*current, member_indent))
            .unwrap_or(self.lines.len());
        let body_start = inline_body.unwrap_or(line.end);
        let body_end = self
            .lines
            .get(next.saturating_sub(1))
            .map_or(body_start, |last| last.end);
        member.body = Some(body_start..body_end.max(body_start));
        let closes = [self.syntax.ruby, self.text(next) == self.syntax.closing] == [true, true];
        next + usize::from(closes)
    }

    fn belongs_to_method(&self, index: usize, member_indent: usize) -> bool {
        [
            self.text(index).is_empty(),
            self.indent(index) > member_indent,
        ]
        .contains(&true)
    }
}
