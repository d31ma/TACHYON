use super::contract_rewrite::SourceEdit;
use super::contract_role::ContractRole;
use super::fsharp_class_type::FsharpClassType;
use super::fsharp_contract::FsharpContract;
use crate::behavior::HttpClientSource;
use std::ops::Range;
use std::path::Path;

pub(crate) struct FsharpContractRewrite<'a> {
    source: &'a str,
    lines: Vec<Range<usize>>,
    code: String,
}

impl<'a> FsharpContractRewrite<'a> {
    const EXTENSION: &'static str = "fs";
    const ATTRIBUTE_OPEN: &'static str = "[<";
    const TYPE_WORD: &'static str = "type ";
    const CONTEXT_MEMBER: &'static str = "member val behaviors";
    const ABSTRACT: &'static str = "abstract ";
    const NAME_JOINER: char = '_';

    pub(crate) fn edits(path: &Path, source: &'a str) -> Option<Result<Vec<SourceEdit>, String>> {
        let fsharp = path.extension().and_then(|value| value.to_str()) == Some(Self::EXTENSION);
        fsharp.then(|| Self::new(source).all_edits())
    }

    fn new(source: &'a str) -> Self {
        let mask = crate::lexical::code_mask(Self::EXTENSION, source);
        let lines = source
            .split_inclusive('\n')
            .scan(0, |offset, line| {
                let start = *offset;
                *offset += line.len();
                Some(start..*offset)
            })
            .collect();
        Self {
            source,
            lines,
            code: HttpClientSource::masked(source, &mask.code),
        }
    }

    fn code_line(&self, index: usize) -> &str {
        &self.code[self.lines[index].clone()]
    }

    fn all_edits(&self) -> Result<Vec<SourceEdit>, String> {
        let nested: Vec<Vec<SourceEdit>> = (0..self.lines.len())
            .filter_map(|index| self.class_type(index))
            .map(|class_type| self.type_edits(&class_type))
            .collect::<Result<_, _>>()?;
        Ok(nested.into_iter().flatten().collect())
    }

    fn marked_role(&self, index: usize) -> Option<ContractRole> {
        let line = self.code_line(index);
        line.starts_with(Self::ATTRIBUTE_OPEN).then_some(())?;
        FsharpContract::attribute_names(line)
            .iter()
            .find_map(|name| ContractRole::of_marker(name))
    }

    fn class_type(&self, marker: usize) -> Option<FsharpClassType> {
        let role = self.marked_role(marker)?;
        let header = (marker + 1..self.lines.len()).find(|index| self.is_code(*index))?;
        let name = Self::class_name(self.code_line(header).trim_end())?;
        let body = (header + 1..self.lines.len())
            .take_while(|index| self.is_inside(*index))
            .collect();
        Some(FsharpClassType {
            role,
            marker: self.code_line(marker).to_owned(),
            name,
            header,
            body,
        })
    }

    fn is_code(&self, index: usize) -> bool {
        let line = self.code_line(index);
        [
            line.trim().is_empty(),
            line.starts_with(Self::ATTRIBUTE_OPEN),
        ] == [false, false]
    }

    fn is_inside(&self, index: usize) -> bool {
        let line = self.code_line(index);
        [
            line.trim().is_empty(),
            line.starts_with(char::is_whitespace),
        ]
        .contains(&true)
    }

    fn class_name(header: &str) -> Option<String> {
        let rest = header.strip_prefix(Self::TYPE_WORD)?.trim_start();
        let length = rest
            .find(|character: char| !Self::is_name_character(character))
            .unwrap_or(rest.len());
        let constructed = rest[length..].trim_start().starts_with('(');
        ([length > 0, constructed] == [true, true]).then(|| rest[..length].to_owned())
    }

    fn is_name_character(character: char) -> bool {
        [character.is_alphanumeric(), character == Self::NAME_JOINER].contains(&true)
    }

    fn type_edits(&self, class_type: &FsharpClassType) -> Result<Vec<SourceEdit>, String> {
        Self::entity_check(class_type)?;
        let mut edits = vec![self.header_edit(class_type)];
        let removals: Vec<Option<SourceEdit>> = class_type
            .body
            .iter()
            .map(|index| self.member_edit(class_type, *index))
            .collect::<Result<_, _>>()?;
        edits.extend(removals.into_iter().flatten());
        Ok(edits)
    }

    fn entity_check(class_type: &FsharpClassType) -> Result<(), String> {
        let bare = !class_type.marker.contains('(');
        let missing = [class_type.role.names_entity, bare] == [true, true];
        (!missing).then_some(()).ok_or_else(|| {
            format!(
                "{} is a Store that names no Entity. A Store names the Entity it keeps: move its code into a service, then run the fix again.",
                class_type.name
            )
        })
    }

    fn header_edit(&self, class_type: &FsharpClassType) -> SourceEdit {
        let line = self.lines[class_type.header].clone();
        let text = &self.source[line.clone()];
        let ending = &text[text.trim_end().len()..];
        (
            line,
            format!("{}{} ={ending}", Self::TYPE_WORD, class_type.name),
        )
    }

    fn member_edit(
        &self,
        class_type: &FsharpClassType,
        index: usize,
    ) -> Result<Option<SourceEdit>, String> {
        let text = self.code_line(index).trim();
        let kept = [
            text.is_empty(),
            text.starts_with(Self::ABSTRACT),
            text.starts_with(Self::ATTRIBUTE_OPEN),
        ]
        .contains(&true);
        let context = text.starts_with(Self::CONTEXT_MEMBER);
        match (kept, context) {
            (true, _) => Ok(None),
            (false, true) => Ok(Some((self.lines[index].clone(), String::new()))),
            (false, false) => Err(format!(
                "{} holds `{text}`. {} Then run the fix again.",
                class_type.name, class_type.role.no_implementation
            )),
        }
    }
}
