use super::contract_form::ContractForm;
use super::contract_member::ContractMember;
use super::indented_members::IndentedMembers;
use super::marker_line::MarkerLine;
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

type Parts = (Range<usize>, Range<usize>, usize);
type MemberReader = fn(ContractForm, &str, Range<usize>) -> Vec<ContractMember>;
type BlockReader = fn(&str, usize, usize) -> Parts;

impl ClientDeclaration {
    const CLIENT: &'static str = "httpclient";
    const CLIENT_SUFFIX: &'static str = "Client";
    const SETUP_SUFFIX: &'static str = "Setup";
    const RUBY: &'static str = "rb";
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

    pub(crate) fn is_mentioned(source: &str) -> bool {
        let folded = source.to_ascii_lowercase().replace('_', "");
        [source.contains("HTTPClient"), folded.contains(Self::CLIENT)].contains(&true)
    }

    pub(crate) fn find(form: ContractForm, source: &str, code: &str) -> Vec<Self> {
        let lines = MarkerLine::read(form, source, code);
        (0..lines.len())
            .filter(|index| lines[*index].is_client())
            .filter_map(|index| Self::read(form, code, &lines, index))
            .collect()
    }

    pub(crate) fn members(&self, form: ContractForm, code: &str) -> Vec<ContractMember> {
        let readers: [MemberReader; 2] = [Self::scanned_members, Self::indented_members];
        readers[usize::from(form.is_indented())](form, code, self.body.clone())
    }

    fn scanned_members(form: ContractForm, code: &str, body: Range<usize>) -> Vec<ContractMember> {
        MemberScanner::members(form, code, body)
    }

    fn indented_members(form: ContractForm, code: &str, body: Range<usize>) -> Vec<ContractMember> {
        IndentedMembers::members(code, body, form.language == Self::RUBY)
    }

    pub(crate) fn option(&self, key: &str) -> Option<&str> {
        self.options.get(key).and_then(Value::as_str)
    }

    pub(crate) fn adapter(&self) -> String {
        self.option("adapter")
            .map_or_else(|| HttpClientSource::adapter_name(&self.name), str::to_owned)
    }

    pub(crate) fn setup(&self) -> String {
        let stem = self
            .name
            .strip_suffix(Self::CLIENT_SUFFIX)
            .unwrap_or(&self.name);
        self.option("setup")
            .map_or_else(|| format!("{stem}{}", Self::SETUP_SUFFIX), str::to_owned)
    }

    pub(crate) fn matching_brace(code: &str, open: usize) -> Option<usize> {
        code.bytes()
            .enumerate()
            .skip(open)
            .scan(0_i32, |depth, (offset, byte)| {
                *depth += i32::from(byte == b'{') - i32::from(byte == b'}');
                Some((*depth, offset))
            })
            .find(|(depth, _)| *depth == 0)
            .map(|(_, offset)| offset)
    }

    fn read(form: ContractForm, code: &str, lines: &[MarkerLine], index: usize) -> Option<Self> {
        let first = Self::first_of_block(code, lines, index);
        let last = Self::last_of_block(code, lines, index);
        let declaration = Self::next_code_line(code, lines[last].end)?;
        let line_end = code[declaration..]
            .find('\n')
            .map_or(code.len(), |at| declaration + at);
        let name = crate::annotation_layout::declaration_names(
            std::path::Path::new(&format!("contract.{}", form.language)),
            &code[declaration..line_end],
        )
        .unwrap_or_default()
        .into_iter()
        .next()?;
        let other_role = lines[first..=last]
            .iter()
            .map(|marker| HttpClientSource::normalized(&marker.name))
            .find(|marker| Self::ROLES.contains(&marker.as_str()));
        let blocks: [BlockReader; 2] = [Self::braced_block, Self::indented_block];
        let (header, body, close) =
            blocks[usize::from(form.is_indented())](code, declaration, line_end);
        Some(Self {
            name,
            marker_at: lines[index].start,
            options: Self::options(&lines[index].arguments),
            other_role,
            header,
            body,
            close,
        })
    }

    fn options(arguments: &str) -> Value {
        let text = arguments.trim();
        Some(text)
            .filter(|text| text.starts_with('{'))
            .and_then(|text| serde_json::from_str(text).ok())
            .unwrap_or(Value::Null)
    }

    fn adjacent(code: &str, lines: &[MarkerLine], earlier: usize, later: usize) -> bool {
        code[lines[earlier].end..lines[later].start]
            .trim()
            .is_empty()
    }

    fn first_of_block(code: &str, lines: &[MarkerLine], index: usize) -> usize {
        let joined = (1..=index)
            .rev()
            .take_while(|at| Self::adjacent(code, lines, *at - 1, *at))
            .count();
        index - joined
    }

    fn last_of_block(code: &str, lines: &[MarkerLine], index: usize) -> usize {
        let joined = (index..lines.len().saturating_sub(1))
            .take_while(|at| Self::adjacent(code, lines, *at, *at + 1))
            .count();
        index + joined
    }

    fn next_code_line(code: &str, from: usize) -> Option<usize> {
        code[from..]
            .split_inclusive('\n')
            .scan(from, |offset, line| {
                let start = *offset + line.len() - line.trim_start().len();
                *offset += line.len();
                Some((start, line))
            })
            .find(|(_, line)| !line.trim().is_empty())
            .map(|(start, _)| start)
    }

    fn braced_block(code: &str, declaration: usize, line_end: usize) -> Parts {
        code[declaration..]
            .find(['{', ';'])
            .map(|at| declaration + at)
            .filter(|at| code.as_bytes()[*at] == b'{')
            .map_or(
                (declaration..line_end, line_end..line_end, line_end),
                |open| Self::braced_parts(code, declaration, open),
            )
    }

    fn braced_parts(code: &str, declaration: usize, open: usize) -> Parts {
        let close = Self::matching_brace(code, open).unwrap_or(code.len());
        (declaration..open, open + 1..close, close)
    }

    fn ends_block(line: &str, indent: usize) -> bool {
        let depth = line.len() - line.trim_start().len();
        [line.trim().is_empty(), depth <= indent] == [false, true]
    }

    fn indented_block(code: &str, declaration: usize, line_end: usize) -> Parts {
        let line_start = code[..declaration].rfind('\n').map_or(0, |at| at + 1);
        let indent = declaration - line_start;
        let body_start = (line_end + 1).min(code.len());
        let end = code[body_start..]
            .split_inclusive('\n')
            .scan(body_start, |offset, line| {
                let start = *offset;
                *offset += line.len();
                Some((start, line))
            })
            .find(|(_, line)| Self::ends_block(line, indent))
            .map_or(code.len(), |(start, _)| start);
        (declaration..line_end, body_start..end, end)
    }
}
