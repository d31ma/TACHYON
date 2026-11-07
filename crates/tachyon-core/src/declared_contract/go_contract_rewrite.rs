use super::contract_declaration::ContractDeclaration;
use super::contract_rewrite::{ContractRewrite, SourceEdit};
use super::go_method::GoMethod;

pub(crate) struct GoContractRewrite<'r, 'a> {
    rewrite: &'r ContractRewrite<'a>,
    lines: Vec<(usize, usize)>,
}

impl<'r, 'a> GoContractRewrite<'r, 'a> {
    const MARKER: &'static str = "// @";
    const MASKED_MARKER: &'static str = "@";
    const START_MARKER: &'static str = "// @OnStart";
    const METHOD: &'static str = "func (";

    pub(crate) fn edits(
        rewrite: &'r ContractRewrite<'a>,
        declaration: &ContractDeclaration,
    ) -> Result<Vec<SourceEdit>, String> {
        let lines = rewrite
            .code
            .split_inclusive('\n')
            .scan(0, |offset, line| {
                let start = *offset;
                *offset += line.len();
                Some((start, *offset))
            })
            .collect();
        let go = Self { rewrite, lines };
        go.contract_edits(declaration)
    }

    fn contract_edits(&self, declaration: &ContractDeclaration) -> Result<Vec<SourceEdit>, String> {
        let name = &declaration.name;
        self.refuse_fields(declaration)?;
        let methods = self.methods(name);
        let signatures: Vec<String> = methods
            .iter()
            .filter(|method| !method.starts_context)
            .map(|method| method.declaration(declaration))
            .collect::<Result<_, _>>()?;
        let mut edits: Vec<SourceEdit> = methods
            .iter()
            .map(|method| (method.whole.clone(), String::new()))
            .collect();
        let end = (declaration.close + 1).min(self.rewrite.source.len());
        edits.push((
            declaration.header.start..end,
            format!("type {name} interface {{\n{}}}", signatures.join("\n")),
        ));
        Ok(edits)
    }

    fn is_field(line: &str) -> bool {
        [
            line.is_empty(),
            line.starts_with(ContractRewrite::CONTEXT_FIELD),
            line.starts_with(Self::MARKER),
            line.starts_with(Self::MASKED_MARKER),
        ] == [false, false, false, false]
    }

    fn refuse_fields(&self, declaration: &ContractDeclaration) -> Result<(), String> {
        let fields = self.rewrite.code[declaration.body.clone()]
            .lines()
            .map(str::trim)
            .any(Self::is_field);
        (!fields).then_some(()).ok_or_else(|| {
            format!(
                "{} holds fields. {} Then run the fix again.",
                declaration.name, declaration.role.no_implementation
            )
        })
    }

    fn line(&self, index: usize) -> &str {
        let (start, end) = self.lines[index];
        &self.rewrite.code[start..end]
    }

    fn source_line(&self, index: usize) -> &str {
        let (start, end) = self.lines[index];
        self.rewrite.source[start..end].trim()
    }

    fn receiver(line: &str) -> Option<&str> {
        line.strip_prefix(Self::METHOD)
            .and_then(|rest| rest.split(')').next())
            .and_then(|receiver| receiver.split_whitespace().last())
            .map(|receiver| receiver.trim_start_matches('*'))
    }

    fn methods(&self, name: &str) -> Vec<GoMethod> {
        (0..self.lines.len())
            .filter(|index| Self::receiver(self.line(*index)) == Some(name))
            .filter_map(|index| self.method(index))
            .collect()
    }

    fn is_marker_or_blank(&self, index: usize) -> bool {
        let text = self.source_line(index);
        [text.starts_with(Self::MARKER), text.is_empty()].contains(&true)
    }

    fn markers(&self, index: usize) -> Vec<usize> {
        let mut found: Vec<usize> = (0..index)
            .rev()
            .take_while(|earlier| self.is_marker_or_blank(*earlier))
            .filter(|earlier| self.source_line(*earlier).starts_with(Self::MARKER))
            .collect();
        found.reverse();
        found
    }

    fn body_open(&self, from: usize) -> Option<usize> {
        self.rewrite
            .code
            .bytes()
            .enumerate()
            .skip(from)
            .scan(0_i32, |depth, (at, byte)| {
                *depth += i32::from(byte == b'(') - i32::from(byte == b')');
                Some((*depth, at, byte))
            })
            .find(|(depth, _, byte)| [*depth == 0, *byte == b'{'] == [true, true])
            .map(|(_, at, _)| at)
    }

    fn method(&self, index: usize) -> Option<GoMethod> {
        let code = &self.rewrite.code;
        let (start, _) = self.lines[index];
        let receiver_close = start + code[start..].find(')')?;
        let open = self.body_open(receiver_close + 1)?;
        let close = ContractDeclaration::matching_brace(code, open)?;
        let marker_lines = self.markers(index);
        let markers: Vec<String> = marker_lines
            .iter()
            .map(|marker| self.source_line(*marker).to_owned())
            .collect();
        let whole_start = marker_lines
            .first()
            .map_or(start, |first| self.lines[*first].0);
        let whole_end = code[close..]
            .find('\n')
            .map_or(self.rewrite.source.len(), |at| close + at + 1);
        Some(GoMethod {
            starts_context: markers.iter().any(|marker| marker == Self::START_MARKER),
            markers,
            signature: self.rewrite.source[receiver_close + 1..open]
                .trim()
                .to_owned(),
            has_body: !code[open + 1..close].trim().is_empty(),
            whole: whole_start..whole_end,
        })
    }
}
