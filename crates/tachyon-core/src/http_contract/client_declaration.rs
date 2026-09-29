use super::contract_form::ContractForm;
use super::contract_member::ContractMember;
use super::indented_members::IndentedMembers;
use super::member_scanner::MemberScanner;
use crate::behavior::HttpClientSource;
use serde_json::Value;
use std::ops::Range;

#[derive(Clone, Debug)]
pub(crate) struct ClientDeclaration {
    pub(crate) name: String,
    pub(crate) marker_at: usize,
    pub(crate) options: Value,
    pub(crate) other_role: Option<String>,
    pub(crate) header: Range<usize>,
    pub(crate) body: Range<usize>,
    pub(crate) close: usize,
}

struct MarkerLine {
    name: String,
    arguments: String,
    start: usize,
    end: usize,
}

impl ClientDeclaration {
    const CLIENT: &'static str = "httpclient";
    const ROLES: [&'static str; 11] = [
        "service",
        "store",
        "helper",
        "converter",
        "setup",
        "entity",
        "controller",
        "requestroute",
        "socketroute",
        "identityprovider",
        "errorhandler",
    ];

    pub(crate) fn find(form: ContractForm, source: &str, code: &str) -> Vec<Self> {
        let lines = Self::marker_lines(form, source, code);
        let mut found = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if HttpClientSource::normalized(&line.name) != Self::CLIENT {
                continue;
            }
            if let Some(declaration) = Self::read(form, code, &lines, index) {
                found.push(declaration);
            }
        }
        found
    }

    pub(crate) fn members(&self, form: ContractForm, code: &str) -> Vec<ContractMember> {
        if form.is_indented() {
            return IndentedMembers::members(code, self.body.clone(), form.language == "rb");
        }
        MemberScanner::members(form, code, self.body.clone())
    }

    pub(crate) fn option(&self, key: &str) -> Option<&str> {
        self.options.get(key).and_then(Value::as_str)
    }

    pub(crate) fn adapter(&self) -> String {
        self.option("adapter")
            .map_or_else(|| HttpClientSource::adapter_name(&self.name), str::to_owned)
    }

    pub(crate) fn setup(&self) -> String {
        if let Some(setup) = self.option("setup") {
            return setup.to_owned();
        }
        let stem = self.name.strip_suffix("Client").unwrap_or(&self.name);
        format!("{stem}Setup")
    }

    fn read(form: ContractForm, code: &str, lines: &[MarkerLine], index: usize) -> Option<Self> {
        let line = &lines[index];
        let (block_start, block_end) = Self::block(code, lines, index);
        let declaration = Self::next_code_line(code, lines[block_end].end)?;
        let line_end = code[declaration..]
            .find('\n')
            .map_or(code.len(), |at| declaration + at);
        let names = crate::annotation_layout::declaration_names(
            std::path::Path::new(&format!("contract.{}", form.language)),
            &code[declaration..line_end],
        )
        .unwrap_or_default();
        let name = names.into_iter().next()?;
        let other_role = lines[block_start..=block_end]
            .iter()
            .map(|marker| HttpClientSource::normalized(&marker.name))
            .find(|marker| Self::ROLES.contains(&marker.as_str()));
        let (header, body, close) = if form.is_indented() {
            Self::indented_block(code, declaration, line_end)
        } else {
            Self::braced_block(code, declaration, line_end)
        };
        Some(Self {
            name,
            marker_at: line.start,
            options: Self::options(&line.arguments),
            other_role,
            header,
            body,
            close,
        })
    }

    fn options(arguments: &str) -> Value {
        let text = arguments.trim();
        if text.starts_with('{') {
            return serde_json::from_str(text).unwrap_or(Value::Null);
        }
        Value::Null
    }

    fn block(code: &str, lines: &[MarkerLine], index: usize) -> (usize, usize) {
        let mut first = index;
        while first > 0
            && code[lines[first - 1].end..lines[first].start]
                .trim()
                .is_empty()
        {
            first -= 1;
        }
        let mut last = index;
        while last + 1 < lines.len()
            && code[lines[last].end..lines[last + 1].start]
                .trim()
                .is_empty()
        {
            last += 1;
        }
        (first, last)
    }

    fn next_code_line(code: &str, from: usize) -> Option<usize> {
        let mut offset = from;
        for line in code[from..].split_inclusive('\n') {
            let start = offset + line.len() - line.trim_start().len();
            offset += line.len();
            if !line.trim().is_empty() {
                return Some(start);
            }
        }
        None
    }

    fn braced_block(
        code: &str,
        declaration: usize,
        line_end: usize,
    ) -> (Range<usize>, Range<usize>, usize) {
        let open = code[declaration..]
            .find(['{', ';'])
            .map(|at| declaration + at)
            .filter(|at| code.as_bytes()[*at] == b'{');
        let Some(open) = open else {
            return (declaration..line_end, line_end..line_end, line_end);
        };
        let close = Self::matching_brace(code, open).unwrap_or(code.len());
        (declaration..open, open + 1..close, close)
    }

    pub(crate) fn matching_brace(code: &str, open: usize) -> Option<usize> {
        let mut depth = 0_i32;
        for (offset, byte) in code.bytes().enumerate().skip(open) {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(offset);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn indented_block(
        code: &str,
        declaration: usize,
        line_end: usize,
    ) -> (Range<usize>, Range<usize>, usize) {
        let line_start = code[..declaration].rfind('\n').map_or(0, |at| at + 1);
        let indent = declaration - line_start;
        let body_start = (line_end + 1).min(code.len());
        let mut offset = body_start;
        for line in code[body_start..].split_inclusive('\n') {
            let depth = line.len() - line.trim_start().len();
            if !line.trim().is_empty() && depth <= indent {
                return (declaration..line_end, body_start..offset, offset);
            }
            offset += line.len();
        }
        (declaration..line_end, body_start..code.len(), code.len())
    }

    fn marker_lines(form: ContractForm, source: &str, code: &str) -> Vec<MarkerLine> {
        let prefix = form.marker_prefix();
        let mut lines = Vec::new();
        let mut offset = 0;
        let mut depth = 0_usize;
        for line in code.split_inclusive('\n') {
            let start = offset + line.len() - line.trim_start().len();
            let end = offset + line.trim_end().len();
            offset += line.len();
            let top_level = depth == 0;
            if !form.is_indented() {
                depth = crate::annotation_layout::brace_depth_after(line, depth);
            }
            let indented = form.is_indented() && start > offset - line.len();
            if !top_level || indented {
                continue;
            }
            let text = &code[start..end.max(start)];
            let Some(rest) = text.strip_prefix(prefix) else {
                continue;
            };
            let length = rest
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
                .count();
            if length == 0 {
                continue;
            }
            let after = start + prefix.len() + length;
            lines.push(MarkerLine {
                name: rest[..length].to_owned(),
                arguments: Self::arguments(&source[after..end.max(after)]),
                start,
                end,
            });
        }
        lines
    }

    fn arguments(tail: &str) -> String {
        let (Some(open), Some(close)) = (tail.find('('), tail.rfind(')')) else {
            return String::new();
        };
        if close <= open {
            return String::new();
        }
        tail[open + 1..close].trim().to_owned()
    }
}
