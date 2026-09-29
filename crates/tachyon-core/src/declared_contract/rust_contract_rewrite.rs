use super::contract_declaration::ContractDeclaration;
use super::contract_rewrite::{ContractRewrite, SourceEdit};
use std::ops::Range;

pub(crate) struct RustContractRewrite;

impl RustContractRewrite {
    const IMPL: &'static str = "impl";
    const FOR: &'static str = "for";
    const STRUCT: &'static str = "struct";
    const ATTRIBUTE: &'static str = "#[";
    const TERMINATORS: [&'static str; 2] = [";", ""];

    pub(crate) fn edits(
        rewrite: &ContractRewrite<'_>,
        declaration: &ContractDeclaration,
    ) -> Result<Vec<SourceEdit>, String> {
        let name = &declaration.name;
        let (impl_header, impl_body) = Self::impl_block(&rewrite.code, name).ok_or_else(|| {
            format!("{name} has no `impl {name}` block. Write it as a trait by hand.")
        })?;
        Self::refuse_state(&rewrite.code[declaration.body.clone()], declaration)?;
        let prefix = rewrite.source[declaration.header.clone()]
            .split(Self::STRUCT)
            .next()
            .unwrap_or("");
        let mut edits = vec![
            (
                declaration.header.start..Self::struct_end(rewrite, declaration),
                String::new(),
            ),
            (impl_header, format!("{prefix}trait {name} ")),
        ];
        let body = ContractDeclaration {
            body: impl_body,
            ..declaration.clone()
        };
        let members: Vec<SourceEdit> = body
            .members(rewrite.form, &rewrite.code)
            .iter()
            .map(|member| rewrite.member_edit(declaration, member))
            .map(|edit| edit.map(|(range, text)| (range, Self::terminated(&text))))
            .collect::<Result<_, _>>()?;
        edits.extend(members);
        Ok(edits)
    }

    fn terminated(text: &str) -> String {
        format!("{text}{}", Self::TERMINATORS[usize::from(text.is_empty())])
    }

    fn struct_end(rewrite: &ContractRewrite<'_>, declaration: &ContractDeclaration) -> usize {
        let after_close = (declaration.close + 1).min(rewrite.source.len());
        let after_line = rewrite.code[after_close..]
            .find('\n')
            .map_or(rewrite.source.len(), |at| after_close + at + 1);
        let braced = rewrite.code.as_bytes().get(declaration.close) == Some(&b'}');
        [after_close, after_line][usize::from(braced)]
    }

    fn is_state_field(field: &str) -> bool {
        let named = Self::without_attributes(field);
        [
            named.is_empty(),
            named.starts_with(ContractRewrite::CONTEXT_FIELD),
        ] == [false, false]
    }

    fn without_attributes(field: &str) -> &str {
        field
            .lines()
            .map(str::trim)
            .find(|line| !line.starts_with(Self::ATTRIBUTE))
            .unwrap_or("")
    }

    fn refuse_state(fields: &str, declaration: &ContractDeclaration) -> Result<(), String> {
        let state = fields.split(',').map(str::trim).any(Self::is_state_field);
        (!state).then_some(()).ok_or_else(|| {
            format!(
                "{} holds fields. {} Then run the fix again.",
                declaration.name, declaration.role.no_implementation
            )
        })
    }

    fn is_own_impl(line: &str, name: &str) -> bool {
        let words: Vec<&str> = line.split_whitespace().collect();
        let names_type = words
            .get(1)
            .is_some_and(|word| word.trim_end_matches('{') == name);
        [
            words.first() == Some(&Self::IMPL),
            names_type,
            words.contains(&Self::FOR),
        ] == [true, true, false]
    }

    fn impl_block(code: &str, name: &str) -> Option<(Range<usize>, Range<usize>)> {
        let (start, line) = code
            .split_inclusive('\n')
            .scan(0, |offset, line| {
                let start = *offset;
                *offset += line.len();
                Some((start, line))
            })
            .find(|(_, line)| Self::is_own_impl(line, name))?;
        let open = start + line.find('{')?;
        let close = ContractDeclaration::matching_brace(code, open)?;
        Some((start..open, open + 1..close))
    }
}
