use super::contract_declaration::ContractDeclaration;
use super::contract_form::ContractForm;
use super::contract_member::ContractMember;
use crate::behavior::HttpClientSource;
use std::ops::Range;

pub(crate) struct MemberScanner<'a> {
    form: ContractForm,
    code: &'a str,
    at: usize,
    end: usize,
}

impl<'a> MemberScanner<'a> {
    const OTHER: u8 = 0;
    const OPEN: u8 = 1;
    const CLOSE: u8 = 2;
    const SKIP: u8 = 3;
    const BODY: u8 = 4;
    const TERMINATOR: u8 = 5;
    const EXPRESSION_END: u8 = 6;
    const ASSIGNMENT: u8 = 7;
    const LINE_END: u8 = 8;
    const EXPRESSION_LINE_END: u8 = 9;
    const OPENERS: [u8; 3] = *b"([{";
    const CLOSERS: [u8; 3] = *b")]}";
    const NAME_PUNCTUATION: [u8; 2] = *b"_:";
    const COMPARISONS: [u8; 4] = *b"=!<>";
    const TYPE_POSITIONS: [u8; 5] = *b"<:|&,";
    const BRACKET: char = '[';
    const ARROW: &'static str = "->";
    const KOTLIN: &'static str = "kt";

    pub(crate) fn members(
        form: ContractForm,
        code: &'a str,
        body: Range<usize>,
    ) -> Vec<ContractMember> {
        let mut scanner = Self {
            form,
            code,
            at: body.start,
            end: body.end,
        };
        scanner.read_all()
    }

    fn read_all(&mut self) -> Vec<ContractMember> {
        let mut members = Vec::new();
        let mut pending = ContractMember::default();
        self.skip_space();
        while self.at < self.end {
            self.read_one(&mut pending, &mut members);
            self.skip_space();
        }
        members
    }

    fn read_one(&mut self, pending: &mut ContractMember, members: &mut Vec<ContractMember>) {
        match self.marker() {
            Some(name) => self.take_marker(pending, &name),
            None => self.take_member(pending, members),
        }
    }

    fn take_marker(&mut self, pending: &mut ContractMember, name: &str) {
        pending.first_marker.get_or_insert(self.line_start(self.at));
        pending.markers.push(HttpClientSource::normalized(name));
        self.skip_marker(name);
    }

    fn take_member(&mut self, pending: &mut ContractMember, members: &mut Vec<ContractMember>) {
        let mut member = std::mem::take(pending);
        self.member(&mut member);
        members.push(member);
    }

    fn byte(&self, at: usize) -> u8 {
        self.code.as_bytes().get(at).copied().unwrap_or(b' ')
    }

    fn is_blank(&self) -> bool {
        let byte = self.byte(self.at);
        [byte.is_ascii_whitespace(), byte == b';'].contains(&true)
    }

    fn skip_space(&mut self) {
        while [self.at < self.end, self.is_blank()] == [true, true] {
            self.at += 1;
        }
    }

    fn line_start(&self, at: usize) -> usize {
        self.code[..at].rfind('\n').map_or(0, |line| line + 1)
    }

    fn is_name_byte(byte: u8) -> bool {
        [
            byte.is_ascii_alphanumeric(),
            Self::NAME_PUNCTUATION.contains(&byte),
        ]
        .contains(&true)
    }

    fn marker(&self) -> Option<String> {
        let rest = self.code[self.at..self.end].strip_prefix(self.form.marker_prefix())?;
        let length = rest
            .bytes()
            .take_while(|byte| Self::is_name_byte(*byte))
            .count();
        let starts_with_letter = rest
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic());
        ([length > 0, starts_with_letter] == [true, true]).then(|| rest[..length].to_owned())
    }

    fn skip_marker(&mut self, name: &str) {
        let bracket = self.form.marker_prefix().find(Self::BRACKET);
        let after_name = self.at + self.form.marker_prefix().len() + name.len();
        self.at = bracket.map_or_else(
            || self.after_optional_group(after_name),
            |index| self.after_group(self.at + index),
        );
    }

    fn after_optional_group(&self, at: usize) -> usize {
        let opens = self.byte(at) == b'(';
        let movers: [fn(&Self, usize) -> usize; 2] = [Self::stay, Self::after_group];
        movers[usize::from(opens)](self, at)
    }

    fn stay(&self, at: usize) -> usize {
        at.min(self.end)
    }

    fn depth_change(&self, at: usize) -> i32 {
        let byte = self.byte(at);
        i32::from(Self::OPENERS.contains(&byte)) - i32::from(Self::CLOSERS.contains(&byte))
    }

    fn after_group(&self, open: usize) -> usize {
        (open..self.end)
            .scan(0_i32, |depth, offset| {
                *depth += self.depth_change(offset);
                Some((*depth, offset))
            })
            .find(|(depth, _)| *depth == 0)
            .map_or(self.end, |(_, offset)| offset + 1)
    }

    fn member(&mut self, member: &mut ContractMember) {
        member.start = self.at;
        let mut depth = 0_i32;
        let mut closed = false;
        while [self.at < self.end, closed] == [true, false] {
            let kind = self.kind(depth, member);
            closed = self.step(kind, member, &mut depth);
        }
        self.finish(closed, member);
    }

    fn kind(&self, depth: i32, member: &ContractMember) -> u8 {
        let top = depth == 0;
        match self.byte(self.at) {
            b'(' | b'[' => Self::OPEN,
            b')' | b']' => Self::CLOSE,
            b'{' if top && member.expression => Self::SKIP,
            b'{' if top && self.opens_type_literal() => Self::SKIP,
            b'{' if top => Self::BODY,
            b';' if top && member.expression => Self::EXPRESSION_END,
            b';' if top => Self::TERMINATOR,
            b'=' if top && !member.expression && self.is_assignment() => Self::ASSIGNMENT,
            b'\n' if top && !member.expression && !self.continues_on_next_line(member) => {
                Self::LINE_END
            }
            b'\n' if top && !self.continues_on_next_line(member) => Self::EXPRESSION_LINE_END,
            _ => Self::OTHER,
        }
    }

    fn step(&mut self, kind: u8, member: &mut ContractMember, depth: &mut i32) -> bool {
        match kind {
            Self::OPEN => self.open_group(member, depth),
            Self::CLOSE => self.close_group(depth),
            Self::SKIP => self.skip_group(),
            Self::BODY => self.block(member),
            Self::TERMINATOR => self.terminate(member),
            Self::EXPRESSION_END => self.pass_end(),
            Self::ASSIGNMENT => self.assign(member),
            Self::LINE_END => self.end_line(member),
            Self::EXPRESSION_LINE_END => true,
            _ => self.pass(),
        }
    }

    fn pass(&mut self) -> bool {
        self.at += 1;
        false
    }

    fn pass_end(&mut self) -> bool {
        self.at += 1;
        true
    }

    fn open_group(&mut self, member: &mut ContractMember, depth: &mut i32) -> bool {
        let call = [self.byte(self.at) == b'(', *depth == 0, member.expression];
        member.method |= call == [true, true, false];
        *depth += 1;
        self.pass()
    }

    fn close_group(&mut self, depth: &mut i32) -> bool {
        *depth -= 1;
        self.pass()
    }

    fn skip_group(&mut self) -> bool {
        self.at = self.after_group(self.at);
        false
    }

    fn block(&mut self, member: &mut ContractMember) -> bool {
        member.end = self.signature_end(member.start);
        let close = ContractDeclaration::matching_brace(self.code, self.at).unwrap_or(self.end);
        member.body = Some(self.at + 1..close.min(self.end));
        self.at = close + 1;
        true
    }

    fn terminate(&mut self, member: &mut ContractMember) -> bool {
        member.end = self.signature_end(member.start);
        member.terminator = Some(self.at);
        self.pass_end()
    }

    fn assign(&mut self, member: &mut ContractMember) -> bool {
        member.end = self.signature_end(member.start);
        member.expression = true;
        self.pass()
    }

    fn end_line(&self, member: &mut ContractMember) -> bool {
        member.end = self.signature_end(member.start);
        true
    }

    fn finish(&self, closed: bool, member: &mut ContractMember) {
        if [closed, member.expression] == [false, false] {
            member.end = self.signature_end(member.start);
        }
    }

    fn signature_end(&self, start: usize) -> usize {
        start + self.code[start..self.at].trim_end().len()
    }

    fn opens_type_literal(&self) -> bool {
        let before = self.code[..self.at].trim_end();
        before
            .bytes()
            .last()
            .is_some_and(|byte| Self::TYPE_POSITIONS.contains(&byte))
    }

    fn is_assignment(&self) -> bool {
        let previous = self.byte(self.at.saturating_sub(1));
        let next = self.byte(self.at + 1);
        [Self::COMPARISONS.contains(&previous), next == b'='] == [false, false]
    }

    fn continues_on_next_line(&self, member: &ContractMember) -> bool {
        let next = self.code[self.at..self.end].trim_start();
        let opens_body = [next.starts_with('{'), member.expression] == [true, false];
        let kotlin_type =
            [next.starts_with(':'), self.form.language == Self::KOTLIN] == [true, true];
        let open_statement = [member.expression, self.form.ends_with_semicolon()] == [true, true];
        [
            opens_body,
            next.starts_with(Self::ARROW),
            kotlin_type,
            open_statement,
        ]
        .contains(&true)
    }
}
