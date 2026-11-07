use super::contract_form::ContractForm;
use super::contract_role::ContractRole;
use crate::behavior::HttpClientSource;

pub(crate) struct MarkerLine {
    pub(crate) name: String,
    pub(crate) arguments: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl MarkerLine {
    const NAME_PUNCTUATION: [u8; 2] = *b"_:";

    pub(crate) fn read(form: ContractForm, source: &str, code: &str) -> Vec<Self> {
        code.split_inclusive('\n')
            .scan((0_usize, 0_usize), |(offset, depth), line| {
                let line_start = *offset;
                let top = *depth == 0;
                *offset += line.len();
                *depth = Self::depth_after(form, line, *depth);
                Some((line_start, line, top))
            })
            .filter(|(_, line, top)| Self::is_candidate(form, line, *top))
            .filter_map(|(line_start, line, _)| Self::parse(form, source, line_start, line))
            .collect()
    }

    pub(crate) fn role(&self) -> Option<ContractRole> {
        ContractRole::of_marker(&HttpClientSource::normalized(&self.name))
    }

    fn depth_after(form: ContractForm, line: &str, depth: usize) -> usize {
        crate::annotation_layout::brace_depth_after(line, depth) * usize::from(!form.is_indented())
    }

    fn is_candidate(form: ContractForm, line: &str, top: bool) -> bool {
        let indented = [form.is_indented(), line.starts_with(char::is_whitespace)] == [true, true];
        [top, indented] == [true, false]
    }

    fn is_name_byte(byte: u8) -> bool {
        [
            byte.is_ascii_alphanumeric(),
            Self::NAME_PUNCTUATION.contains(&byte),
        ]
        .contains(&true)
    }

    fn parse(form: ContractForm, source: &str, line_start: usize, line: &str) -> Option<Self> {
        let start = line_start + line.len() - line.trim_start().len();
        let end = (line_start + line.trim_end().len()).max(start);
        let rest = line.trim().strip_prefix(form.marker_prefix())?;
        let length = rest
            .bytes()
            .take_while(|byte| Self::is_name_byte(*byte))
            .count();
        (length > 0).then_some(())?;
        let after = start + form.marker_prefix().len() + length;
        Some(Self {
            name: rest[..length].to_owned(),
            arguments: Self::arguments(&source[after..end.max(after)]),
            start,
            end,
        })
    }

    fn arguments(tail: &str) -> String {
        tail.find('(')
            .zip(tail.rfind(')'))
            .filter(|(open, close)| close > open)
            .map_or_else(String::new, |(open, close)| {
                tail[open + 1..close].trim().to_owned()
            })
    }
}
