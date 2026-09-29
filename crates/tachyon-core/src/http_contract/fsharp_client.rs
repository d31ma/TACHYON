use super::contract_member::ContractMember;
use super::fsharp_declaration::FsharpDeclaration;
use crate::behavior::HttpClientSource;
use std::path::Path;

pub(crate) struct FsharpClient<'a> {
    lines: Vec<&'a str>,
    code: Vec<String>,
}

impl<'a> FsharpClient<'a> {
    const EXTENSION: &'static str = "fs";
    const ATTRIBUTE_OPEN: &'static str = "[<";
    const ATTRIBUTE_CLOSE: &'static str = ">]";
    const CLIENT: &'static str = "httpclient";
    const TYPE_WORD: &'static str = "type ";
    const CONSTRUCTOR: &'static str = "()";
    const DEFAULT_INDENT: &'static str = "    ";
    const NAME_JOINER: char = '_';
    const CONTEXT: &'static str = "member val behaviors : BehaviorContext = Behaviors.create(typeof<{setup}>, \"{partition}\", new BehaviorSignal())";

    pub(crate) fn lower(path: &Path, source: &'a str) -> Option<String> {
        let fsharp = path.extension().and_then(|value| value.to_str()) == Some(Self::EXTENSION);
        fsharp.then_some(())?;
        let client = Self::new(source);
        let declarations = client.declarations();
        (!declarations.is_empty()).then(|| client.lowered(&declarations))
    }

    fn new(source: &'a str) -> Self {
        let mask = crate::lexical::code_mask(Self::EXTENSION, source);
        let masked = HttpClientSource::masked(source, &mask.code);
        Self {
            lines: source.split_inclusive('\n').collect(),
            code: masked.split_inclusive('\n').map(str::to_owned).collect(),
        }
    }

    fn declarations(&self) -> Vec<FsharpDeclaration> {
        (0..self.code.len())
            .filter(|index| self.is_client_marker(*index))
            .filter_map(|index| self.declaration(index))
            .collect()
    }

    fn is_client_marker(&self, index: usize) -> bool {
        let line = &self.code[index];
        let top = line.starts_with(Self::ATTRIBUTE_OPEN);
        let names = Self::attribute_names(line);
        [top, names.iter().any(|name| name == Self::CLIENT)] == [true, true]
    }

    fn attribute_names(line: &str) -> Vec<String> {
        let inside = line
            .trim()
            .trim_start_matches(Self::ATTRIBUTE_OPEN)
            .trim_end_matches(Self::ATTRIBUTE_CLOSE);
        inside
            .split(';')
            .map(|attribute| attribute.split('(').next().unwrap_or(attribute).trim())
            .map(HttpClientSource::normalized)
            .collect()
    }

    fn declaration(&self, marker: usize) -> Option<FsharpDeclaration> {
        let header = (marker + 1..self.code.len()).find(|index| self.is_code(*index))?;
        let name = Self::class_less_name(self.code[header].trim_end())?;
        let body = (header + 1..self.code.len())
            .take_while(|index| self.is_inside(*index))
            .collect();
        Some(FsharpDeclaration {
            header,
            name,
            options: FsharpDeclaration::options_of(self.lines[marker]),
            body,
        })
    }

    fn is_code(&self, index: usize) -> bool {
        let line = &self.code[index];
        [
            line.trim().is_empty(),
            line.starts_with(Self::ATTRIBUTE_OPEN),
        ] == [false, false]
    }

    fn is_inside(&self, index: usize) -> bool {
        let line = &self.code[index];
        [
            line.trim().is_empty(),
            line.starts_with(char::is_whitespace),
        ]
        .contains(&true)
    }

    fn class_less_name(header: &str) -> Option<String> {
        let rest = header.strip_prefix(Self::TYPE_WORD)?.trim_start();
        let length = rest
            .find(|character: char| !Self::is_name_character(character))
            .unwrap_or(rest.len());
        let interface = rest[length..].trim_start().starts_with('=');
        ([length > 0, interface] == [true, true]).then(|| rest[..length].to_owned())
    }

    fn is_name_character(character: char) -> bool {
        [character.is_alphanumeric(), character == Self::NAME_JOINER].contains(&true)
    }

    fn lowered(&self, declarations: &[FsharpDeclaration]) -> String {
        self.lines
            .iter()
            .enumerate()
            .map(|(index, line)| self.lowered_line(declarations, index, line))
            .collect()
    }

    fn lowered_line(&self, declarations: &[FsharpDeclaration], index: usize, line: &str) -> String {
        declarations
            .iter()
            .find(|declaration| declaration.header == index)
            .map_or_else(
                || line.to_owned(),
                |declaration| self.concrete_header(declaration, line),
            )
    }

    fn concrete_header(&self, declaration: &FsharpDeclaration, line: &str) -> String {
        let at = line.find(&declaration.name).unwrap_or(0) + declaration.name.len();
        let header = format!("{}{}{}", &line[..at], Self::CONSTRUCTOR, &line[at..]);
        let context = self.context_line(declaration);
        format!("{header}{context}")
    }

    fn context_line(&self, declaration: &FsharpDeclaration) -> String {
        let sends = declaration
            .body
            .iter()
            .any(|index| Self::sends_request(&self.code[*index]));
        if !sends {
            return String::new();
        }
        let text = Self::CONTEXT
            .replace("{setup}", &declaration.setup())
            .replace("{partition}", &declaration.partition());
        format!("{}{text}\n\n", self.indent(declaration))
    }

    fn sends_request(line: &str) -> bool {
        let trimmed = line.trim_start();
        let marker = trimmed.starts_with(Self::ATTRIBUTE_OPEN);
        let member = ContractMember {
            markers: Self::attribute_names(trimmed),
            ..ContractMember::default()
        };
        [marker, member.sends_request()] == [true, true]
    }

    fn indent(&self, declaration: &FsharpDeclaration) -> String {
        declaration
            .body
            .iter()
            .map(|index| &self.code[*index])
            .find(|line| !line.trim().is_empty())
            .map_or_else(
                || String::from(Self::DEFAULT_INDENT),
                |line| line[..line.len() - line.trim_start().len()].to_owned(),
            )
    }
}
