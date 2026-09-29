use super::client_declaration::ClientDeclaration;
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
        loop {
            self.skip_space();
            if self.at >= self.end {
                break;
            }
            if let Some(name) = self.marker() {
                pending.first_marker.get_or_insert(self.line_start(self.at));
                pending.markers.push(HttpClientSource::normalized(&name));
                self.skip_marker(&name);
                continue;
            }
            let mut member = std::mem::take(&mut pending);
            self.member(&mut member);
            members.push(member);
        }
        members
    }

    fn byte(&self, at: usize) -> u8 {
        self.code.as_bytes().get(at).copied().unwrap_or(b' ')
    }

    fn skip_space(&mut self) {
        while self.at < self.end
            && (self.byte(self.at).is_ascii_whitespace() || self.byte(self.at) == b';')
        {
            self.at += 1;
        }
    }

    fn line_start(&self, at: usize) -> usize {
        self.code[..at].rfind('\n').map_or(0, |line| line + 1)
    }

    fn marker(&self) -> Option<String> {
        let prefix = self.form.marker_prefix();
        let rest = self.code[self.at..self.end].strip_prefix(prefix)?;
        let length = rest
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
            .count();
        let starts_with_letter = rest
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic());
        (length > 0 && starts_with_letter).then(|| rest[..length].to_owned())
    }

    fn skip_marker(&mut self, name: &str) {
        let prefix = self.form.marker_prefix();
        self.at += prefix.len() + name.len();
        if self.byte(self.at) == b'(' {
            self.at = self.after_group(self.at);
        }
        if prefix != "@" {
            while self.at < self.end && self.byte(self.at) != b']' {
                self.at = if self.byte(self.at) == b'(' {
                    self.after_group(self.at)
                } else {
                    self.at + 1
                };
            }
            self.at += 1;
        }
    }

    fn after_group(&self, open: usize) -> usize {
        let mut depth = 0_i32;
        for offset in open..self.end {
            match self.byte(offset) {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return offset + 1;
                    }
                }
                _ => {}
            }
        }
        self.end
    }

    fn member(&mut self, member: &mut ContractMember) {
        member.start = self.at;
        let mut depth = 0_i32;
        while self.at < self.end {
            let byte = self.byte(self.at);
            match byte {
                b'(' | b'[' => {
                    member.method |= byte == b'(' && depth == 0 && !member.expression;
                    depth += 1;
                }
                b')' | b']' => depth -= 1,
                b'{' if depth == 0 && member.expression => {
                    self.at = self.after_group(self.at);
                    continue;
                }
                b'{' if depth == 0 => {
                    self.block(member);
                    return;
                }
                b';' if depth == 0 => {
                    if !member.expression {
                        member.end = self.signature_end(member.start);
                        member.terminator = Some(self.at);
                    }
                    self.at += 1;
                    return;
                }
                b'=' if depth == 0 && !member.expression && self.is_assignment() => {
                    member.end = self.signature_end(member.start);
                    member.expression = true;
                }
                b'\n' if depth == 0 && !self.continues_on_next_line(member) => {
                    self.finish(member);
                    return;
                }
                _ => {}
            }
            self.at += 1;
        }
        self.finish(member);
    }

    fn block(&mut self, member: &mut ContractMember) {
        member.end = self.signature_end(member.start);
        let close = ClientDeclaration::matching_brace(self.code, self.at).unwrap_or(self.end);
        member.body = Some(self.at + 1..close.min(self.end));
        self.at = close + 1;
    }

    fn finish(&self, member: &mut ContractMember) {
        if !member.expression {
            member.end = self.signature_end(member.start);
        }
    }

    fn signature_end(&self, start: usize) -> usize {
        start + self.code[start..self.at].trim_end().len()
    }

    fn is_assignment(&self) -> bool {
        let previous = self.byte(self.at.saturating_sub(1));
        let next = self.byte(self.at + 1);
        !matches!(previous, b'=' | b'!' | b'<' | b'>') && next != b'='
    }

    fn continues_on_next_line(&self, member: &ContractMember) -> bool {
        let next = self.code[self.at..self.end].trim_start();
        let opens_body = next.starts_with('{') && !member.expression;
        let kotlin_type = next.starts_with(':') && self.form.language == "kt";
        let open_statement = member.expression && self.form.ends_with_semicolon();
        opens_body || next.starts_with("->") || kotlin_type || open_statement
    }
}
