use super::client_declaration::ClientDeclaration;
use super::contract_form::ContractForm;
use super::contract_member::ContractMember;
use super::go_contract_rewrite::GoContractRewrite;
use super::rust_contract_rewrite::RustContractRewrite;
use crate::behavior::HttpClientSource;
use std::ops::Range;
use std::path::Path;

pub(crate) struct ContractRewrite<'a> {
    pub(crate) form: ContractForm,
    pub(crate) source: &'a str,
    pub(crate) code: String,
}

pub(crate) type SourceEdit = (Range<usize>, String);
type BodilessEdit<'a> = fn(&ContractRewrite<'a>, &ContractMember, usize) -> SourceEdit;

impl<'a> ContractRewrite<'a> {
    pub(crate) const CONTEXT_FIELD: &'static str = "behaviors";
    const START_MARKER: &'static str = "onstart";
    const RUST_CONSTRUCTOR: &'static str = "new";
    const RUST: &'static str = "rs";
    const GO: &'static str = "go";
    const PYTHON: &'static str = "py";
    const CLASS: &'static str = "class";
    const INTERFACE: &'static str = "interface";
    const CONTRACT_WORDS: [(&'static str, &'static str); 2] =
        [("swift", "protocol"), ("dart", "abstract interface class")];
    const DROPPED_MODIFIERS: [&'static str; 6] =
        ["final", "sealed", "abstract", "open", "data", "static"];
    const CSHARP_METHOD_MODIFIERS: [&'static str; 3] = ["public", "async", "virtual"];
    const PROTOCOL: &'static str = "Protocol";
    const PROTOCOL_IMPORT: &'static str = "from typing import Protocol\n\n";
    const TYPING_IMPORT: &'static str = "from typing import ";
    const DART_ASYNC: &'static str = "async";

    pub(crate) fn edits(path: &Path, source: &'a str) -> Result<Vec<SourceEdit>, String> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        ContractForm::of(extension)
            .filter(|form| form.has_contract())
            .filter(|_| ClientDeclaration::is_mentioned(source))
            .map_or(Ok(Vec::new()), |form| Self::new(form, source).all_edits())
    }

    fn new(form: ContractForm, source: &'a str) -> Self {
        let mask = crate::lexical::code_mask(form.language, source);
        Self {
            form,
            source,
            code: HttpClientSource::masked(source, &mask.code),
        }
    }

    fn all_edits(&self) -> Result<Vec<SourceEdit>, String> {
        let nested: Vec<Vec<SourceEdit>> =
            ClientDeclaration::find(self.form, self.source, &self.code)
                .iter()
                .filter(|declaration| {
                    !self
                        .form
                        .declares_contract(&self.code[declaration.header.clone()])
                })
                .map(|declaration| {
                    Self::role_check(declaration).and_then(|()| self.declaration(declaration))
                })
                .collect::<Result<_, _>>()?;
        Ok(nested.into_iter().flatten().collect())
    }

    fn role_check(declaration: &ClientDeclaration) -> Result<(), String> {
        declaration.other_role.as_ref().map_or(Ok(()), |role| {
            Err(format!(
                "{} is an HTTPClient and a {role}. Move the {role} code into its own class, then run the fix again.",
                declaration.name
            ))
        })
    }

    fn declaration(&self, declaration: &ClientDeclaration) -> Result<Vec<SourceEdit>, String> {
        match self.form.language {
            Self::RUST => RustContractRewrite::edits(self, declaration),
            Self::GO => GoContractRewrite::edits(self, declaration),
            _ => self.braced(declaration),
        }
    }

    fn braced(&self, declaration: &ClientDeclaration) -> Result<Vec<SourceEdit>, String> {
        let mut edits = vec![(declaration.header.clone(), self.header(declaration))];
        let members: Vec<SourceEdit> = declaration
            .members(self.form, &self.code)
            .iter()
            .map(|member| self.member_edit(&declaration.name, member))
            .collect::<Result<_, _>>()?;
        edits.extend(members);
        edits.extend(self.protocol_import());
        Ok(edits)
    }

    fn header(&self, declaration: &ClientDeclaration) -> String {
        let text = &self.source[declaration.header.clone()];
        if self.form.language == Self::PYTHON {
            Self::python_header(text)
        } else {
            self.typed_header(text)
        }
    }

    fn python_header(text: &str) -> String {
        let open = text.find(['(', ':']).unwrap_or(text.len());
        format!("{}({}):", &text[..open], Self::PROTOCOL)
    }

    fn typed_header(&self, text: &str) -> String {
        let contract = Self::CONTRACT_WORDS
            .iter()
            .find(|(language, _)| *language == self.form.language)
            .map_or(Self::INTERFACE, |(_, word)| *word);
        let kept: Vec<&str> = text
            .split_whitespace()
            .filter(|word| !Self::DROPPED_MODIFIERS.contains(word))
            .map(|word| [word, contract][usize::from(word == Self::CLASS)])
            .collect();
        format!("{} ", kept.join(" "))
    }

    fn protocol_import(&self) -> Option<SourceEdit> {
        (self.form.language == Self::PYTHON).then(|| self.import_edit())
    }

    fn import_edit(&self) -> SourceEdit {
        self.source
            .split_inclusive('\n')
            .scan(0, |offset, line| {
                let start = *offset;
                *offset += line.len();
                Some((start, line.trim_end()))
            })
            .find_map(|(start, line)| {
                line.strip_prefix(Self::TYPING_IMPORT)
                    .map(|names| (start + line.len(), names))
            })
            .map_or_else(
                || (0..0, String::from(Self::PROTOCOL_IMPORT)),
                |(end, names)| (end..end, Self::protocol_suffix(names)),
            )
    }

    fn protocol_suffix(names: &str) -> String {
        let imported = names.split(',').any(|name| name.trim() == Self::PROTOCOL);
        ["", ", Protocol"][usize::from(!imported)].to_owned()
    }

    fn is_context_scaffolding(&self, member: &ContractMember) -> bool {
        let name = member.name(&self.code);
        let starts = member
            .markers
            .iter()
            .any(|marker| marker == Self::START_MARKER);
        let context_field = [member.method, name == Self::CONTEXT_FIELD] == [false, true];
        let constructor = [member.method, name == Self::RUST_CONSTRUCTOR] == [true, true];
        [context_field, starts, constructor].contains(&true)
    }

    fn keeps_logic(&self, member: &ContractMember) -> bool {
        [
            !member.method,
            member.has_code(&self.code),
            !member.is_declared_call(),
        ]
        .contains(&true)
    }

    fn removal(&self, member: &ContractMember) -> SourceEdit {
        let first = member.first_marker.unwrap_or(member.start);
        let start = self.code[..first].rfind('\n').map_or(0, |at| at + 1);
        let end = member
            .body
            .as_ref()
            .map_or(member.terminator.map_or(member.end, |at| at + 1), |body| {
                body.end + 1
            })
            .min(self.source.len());
        let end = self.code[end..]
            .find('\n')
            .map_or(self.source.len(), |at| end + at + 1);
        (start..end, String::new())
    }

    pub(crate) fn member_edit(
        &self,
        client: &str,
        member: &ContractMember,
    ) -> Result<SourceEdit, String> {
        self.is_context_scaffolding(member)
            .then(|| self.removal(member))
            .map_or_else(|| self.signature_edit(client, member), Ok)
    }

    fn signature_edit(&self, client: &str, member: &ContractMember) -> Result<SourceEdit, String> {
        (!self.keeps_logic(member))
            .then(|| self.bodiless(member))
            .ok_or_else(|| {
                format!(
                    "{client}.{} holds code or state. An HTTPClient has no implementation: move it into a service, then run the fix again.",
                    member.name(&self.code)
                )
            })
    }

    fn bodiless(&self, member: &ContractMember) -> SourceEdit {
        let python = self.form.language == Self::PYTHON;
        let end = member
            .body
            .as_ref()
            .map_or(member.end, |body| body.end + usize::from(!python));
        let editors: [BodilessEdit<'a>; 2] = [Self::signature_bodiless, Self::python_bodiless];
        editors[usize::from(python)](self, member, end)
    }

    fn signature_bodiless(&self, member: &ContractMember, end: usize) -> SourceEdit {
        (member.start..end, self.signature_only(member))
    }

    fn python_bodiless(&self, member: &ContractMember, end: usize) -> SourceEdit {
        let body = &self.source[member.end..end];
        let blank_lines = body.len() - body.trim_end_matches(['\n', ' ']).len();
        let newlines = body[body.len() - blank_lines..].matches('\n').count();
        (member.end..end, format!(": ...{}", "\n".repeat(newlines)))
    }

    fn signature_only(&self, member: &ContractMember) -> String {
        let signature = self.source[member.start..member.end].trim_end();
        match self.form.language {
            "cs" => format!(
                "{};",
                Self::without_words(signature, &Self::CSHARP_METHOD_MODIFIERS)
            ),
            "dart" => format!(
                "{};",
                signature.trim_end_matches(Self::DART_ASYNC).trim_end()
            ),
            "java" | "php" => format!("{signature};"),
            _ => signature.to_owned(),
        }
    }

    fn without_words(signature: &str, words: &[&str]) -> String {
        let kept: Vec<&str> = signature
            .split_whitespace()
            .skip_while(|word| words.contains(word))
            .collect();
        kept.join(" ")
    }
}
