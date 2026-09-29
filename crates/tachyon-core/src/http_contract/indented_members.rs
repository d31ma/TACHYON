use super::contract_member::ContractMember;
use crate::behavior::HttpClientSource;
use std::ops::Range;

pub(crate) struct IndentedMembers<'a> {
    code: &'a str,
    lines: Vec<Range<usize>>,
    ruby: bool,
}

impl<'a> IndentedMembers<'a> {
    const PYTHON_METHODS: [&'static str; 2] = ["def ", "async def "];
    const RUBY_METHOD: &'static str = "def ";
    const RUBY_END: &'static str = "end";

    pub(crate) fn members(code: &'a str, body: Range<usize>, ruby: bool) -> Vec<ContractMember> {
        let mut lines = Vec::new();
        let mut offset = body.start;
        for line in code[body.clone()].split_inclusive('\n') {
            lines.push(offset..offset + line.len());
            offset += line.len();
        }
        let reader = Self { code, lines, ruby };
        reader.read_all()
    }

    fn indent(&self, index: usize) -> usize {
        let text = &self.code[self.lines[index].clone()];
        text.len() - text.trim_start().len()
    }

    fn text(&self, index: usize) -> &str {
        self.code[self.lines[index].clone()].trim()
    }

    fn read_all(&self) -> Vec<ContractMember> {
        let Some(first) = (0..self.lines.len()).find(|index| !self.text(*index).is_empty()) else {
            return Vec::new();
        };
        let member_indent = self.indent(first);
        let mut members = Vec::new();
        let mut pending = ContractMember::default();
        let mut index = first;
        while index < self.lines.len() {
            let text = self.text(index);
            if text.is_empty() || self.indent(index) != member_indent {
                index += 1;
                continue;
            }
            let start = self.lines[index].start + self.indent(index);
            if let Some(name) = self.marker(text) {
                pending.first_marker.get_or_insert(self.lines[index].start);
                pending.markers.push(HttpClientSource::normalized(&name));
                index = self.after_open_group(index);
                continue;
            }
            let mut member = std::mem::take(&mut pending);
            member.start = start;
            index = if self.is_method(text) {
                self.method(index, member_indent, &mut member)
            } else {
                member.end =
                    self.lines[index].start + self.code[self.lines[index].clone()].trim_end().len();
                index + 1
            };
            members.push(member);
        }
        members
    }

    fn marker(&self, text: &str) -> Option<String> {
        let rest = if self.ruby {
            text.strip_prefix("#[")?
        } else {
            text.strip_prefix('@')?
        };
        let length = rest
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
            .count();
        (length > 0).then(|| rest[..length].rsplit('.').next().unwrap_or("").to_owned())
    }

    fn is_method(&self, text: &str) -> bool {
        if self.ruby {
            return text.starts_with(Self::RUBY_METHOD);
        }
        Self::PYTHON_METHODS
            .iter()
            .any(|prefix| text.starts_with(prefix))
    }

    fn after_open_group(&self, index: usize) -> usize {
        let mut depth = 0_usize;
        let mut current = index;
        while current < self.lines.len() {
            for byte in self.code[self.lines[current].clone()].bytes() {
                match byte {
                    b'(' => depth += 1,
                    b')' => depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
            current += 1;
            if depth == 0 {
                break;
            }
        }
        current
    }

    fn method(&self, index: usize, member_indent: usize, member: &mut ContractMember) -> usize {
        member.method = true;
        let signature_last = self.after_open_group(index) - 1;
        let line = self.lines[signature_last].clone();
        let line_text = &self.code[line.clone()];
        let inline_body = if self.ruby {
            None
        } else {
            line_text.rfind(':').map(|colon| line.start + colon + 1)
        };
        member.end = inline_body.map_or(line.start + line_text.trim_end().len(), |colon| colon - 1);
        let mut next = signature_last + 1;
        while next < self.lines.len() && self.belongs_to_method(next, member_indent) {
            next += 1;
        }
        let body_start = inline_body.unwrap_or(line.end);
        let body_end = self
            .lines
            .get(next.saturating_sub(1))
            .map_or(body_start, |last| last.end);
        let ends_with_keyword =
            self.ruby && next < self.lines.len() && self.text(next) == Self::RUBY_END;
        member.body = Some(body_start..body_end.max(body_start));
        if ends_with_keyword {
            return next + 1;
        }
        next
    }

    fn belongs_to_method(&self, index: usize, member_indent: usize) -> bool {
        self.text(index).is_empty() || self.indent(index) > member_indent
    }
}
