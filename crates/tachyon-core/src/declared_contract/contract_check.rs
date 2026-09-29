use super::contract_declaration::ContractDeclaration;
use super::contract_form::ContractForm;
use super::contract_member::ContractMember;
use super::contract_role::ContractRole;
use crate::Failure;
use crate::annotation_rules::Messages;
use crate::behavior::HttpClientSource;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct ContractCheck<'a> {
    path: &'a Path,
    source: &'a str,
    code: String,
    form: ContractForm,
}

type Located = (usize, String);
type Problem = (usize, String, ContractRole);

impl<'a> ContractCheck<'a> {
    const CODE: u16 = 1308;
    const WRITTEN_MARKERS: [&'static str; 9] = [
        "GetRequest",
        "PostRequest",
        "PutRequest",
        "PatchRequest",
        "DeleteRequest",
        "StreamRequest",
        "SocketRequest",
        "Gateway",
        "Query",
    ];
    const TYPE_ONLY_FIELD: &'static str = "declare ";
    const TYPESCRIPT: &'static str = "ts";
    const GO: &'static str = "go";
    const RUST: &'static str = "rs";
    const GO_METHOD: &'static str = "func (";
    const RUST_IMPL: &'static str = "impl";
    const RUST_GENERIC_IMPL: &'static str = "impl<";

    pub(crate) fn check(path: &'a Path, source: &'a str) -> Result<(), Failure> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        ContractForm::of(extension)
            .filter(|_| ContractRole::is_mentioned(source))
            .map_or(Ok(()), |form| Self::new(path, source, form).checked())
    }

    fn new(path: &'a Path, source: &'a str, form: ContractForm) -> Self {
        let mask = crate::lexical::code_mask(form.language, source);
        Self {
            path,
            source,
            code: HttpClientSource::masked(source, &mask.code),
            form,
        }
    }

    fn checked(&self) -> Result<(), Failure> {
        ContractDeclaration::find(self.form, self.source, &self.code)
            .iter()
            .find_map(|declaration| self.problem(declaration))
            .map_or(Ok(()), |(at, message, role)| {
                Err(self.failure(at, message, role))
            })
    }

    fn problem(&self, declaration: &ContractDeclaration) -> Option<Problem> {
        self.role_problem(declaration)
            .or_else(|| self.entity_problem(declaration))
            .or_else(|| self.contract_problem(declaration))
            .map(|(at, message)| (at, message, declaration.role))
    }

    fn contract_problem(&self, declaration: &ContractDeclaration) -> Option<Located> {
        (!declaration.keeps_class_form(self.form, &self.code))
            .then(|| self.shape_problem(declaration))
            .flatten()
    }

    fn shape_problem(&self, declaration: &ContractDeclaration) -> Option<Located> {
        self.form_problem(declaration)
            .or_else(|| self.member_problem(declaration))
            .or_else(|| self.implementation_problem(declaration))
    }

    fn role_problem(&self, declaration: &ContractDeclaration) -> Option<Located> {
        let role = declaration.role;
        let spelled = Messages::spelled(self.form.language, role.marker);
        let name = &declaration.name;
        declaration.other_role.as_ref().map(|other| {
            (
                declaration.marker_at,
                format!("{name} is marked with {spelled} and with the {other} role. {} is its own role and has no implementation: keep the {} in {name}, and let a service that needs them declare a {name} field", role.titled, role.works),
            )
        })
    }

    fn entity_problem(&self, declaration: &ContractDeclaration) -> Option<Located> {
        let role = declaration.role;
        let spelled = Messages::spelled(self.form.language, role.marker);
        let message = format!(
            "{} is a {spelled} that names no Entity. A Store names the Entity it keeps as its first argument, followed by its options (fields, defaultSize and maxSize), and Tachyon generates each query. Move other logic into a service.",
            declaration.name
        );
        let missing = [role.names_entity, declaration.names_entity] == [true, false];
        missing.then_some((declaration.marker_at, message))
    }

    fn form_problem(&self, declaration: &ContractDeclaration) -> Option<Located> {
        let role = declaration.role;
        let spelled = Messages::spelled(self.form.language, role.marker);
        let header = &self.code[declaration.header.clone()];
        let message = format!(
            "{} is a {spelled}, so declare it as {}. {}",
            declaration.name,
            self.form.contract_words(),
            role.no_implementation
        );
        (!self.form.declares_contract(header)).then_some((declaration.header.start, message))
    }

    fn member_problem(&self, declaration: &ContractDeclaration) -> Option<Located> {
        declaration
            .members(self.form, &self.code)
            .iter()
            .find_map(|member| self.member_issue(declaration, member))
    }

    fn member_issue(
        &self,
        declaration: &ContractDeclaration,
        member: &ContractMember,
    ) -> Option<Located> {
        let name = &declaration.name;
        let role = declaration.role;
        self.constructor_issue(name, role, member)
            .or_else(|| self.state_issue(name, role, member))
            .or_else(|| self.body_issue(name, role, member))
            .or_else(|| self.annotation_issue(name, role, member))
    }

    fn constructor_issue(
        &self,
        name: &str,
        role: ContractRole,
        member: &ContractMember,
    ) -> Option<Located> {
        let message = format!("{name} writes a constructor. {}", role.no_implementation);
        member
            .is_constructor(&self.code)
            .then_some((member.start, message))
    }

    fn state_issue(
        &self,
        name: &str,
        role: ContractRole,
        member: &ContractMember,
    ) -> Option<Located> {
        let text = self.code[member.start..member.end].trim();
        let type_only = [
            text.starts_with(Self::TYPE_ONLY_FIELD),
            self.form.language == Self::TYPESCRIPT,
        ] == [true, true];
        let noun = role.noun;
        let message = format!(
            "{name} holds `{text}`. Tachyon creates the behavior context of a {noun} itself, and a {noun} keeps no other state. {}",
            role.no_implementation
        );
        ([member.method, type_only] == [false, false]).then_some((member.start, message))
    }

    fn body_issue(
        &self,
        name: &str,
        role: ContractRole,
        member: &ContractMember,
    ) -> Option<Located> {
        let braced = [self.form.has_contract(), self.form.is_indented()] == [true, false];
        let written = [member.has_code(&self.code), member.has_block()][usize::from(braced)];
        let message = format!(
            "{name}.{} has a body. {}",
            member.name(&self.code),
            role.no_implementation
        );
        ([member.method, written] == [true, true]).then_some((member.start, message))
    }

    fn annotation_issue(
        &self,
        name: &str,
        role: ContractRole,
        member: &ContractMember,
    ) -> Option<Located> {
        let message = role.member_rule(name, &member.name(&self.code), &self.written_markers(role));
        ([member.method, role.is_call(member, &self.code)] == [true, false])
            .then_some((member.start, message))
    }

    fn written_markers(&self, role: ContractRole) -> String {
        let spelled: Vec<String> = Self::WRITTEN_MARKERS
            .iter()
            .filter(|marker| {
                role.member_markers()
                    .contains(&marker.to_ascii_lowercase().as_str())
            })
            .map(|marker| Messages::spelled(self.form.language, marker))
            .collect();
        spelled.join(", ")
    }

    fn implementation_problem(&self, declaration: &ContractDeclaration) -> Option<Located> {
        let name = &declaration.name;
        self.code
            .split_inclusive('\n')
            .scan(0, |offset, line| {
                let start = *offset;
                *offset += line.len();
                Some((start, line))
            })
            .find(|(_, line)| self.implements(line.trim(), name))
            .map(|(start, _)| {
                (
                    start,
                    format!(
                        "This file implements {name} by hand. {}",
                        declaration.role.no_implementation
                    ),
                )
            })
    }

    fn implements(&self, line: &str, name: &str) -> bool {
        match self.form.language {
            Self::GO => Self::go_implements(line, name),
            Self::RUST => Self::rust_implements(line, name),
            _ => false,
        }
    }

    fn go_implements(line: &str, name: &str) -> bool {
        line.strip_prefix(Self::GO_METHOD)
            .and_then(|rest| rest.split(')').next())
            .and_then(|receiver| receiver.split_whitespace().last())
            .is_some_and(|receiver| receiver.trim_start_matches('*') == name)
    }

    fn rust_implements(line: &str, name: &str) -> bool {
        let words: Vec<&str> = line.split_whitespace().collect();
        let first = words.first().copied().unwrap_or("");
        let starts_impl = [
            first == Self::RUST_IMPL,
            first.starts_with(Self::RUST_GENERIC_IMPL),
        ]
        .contains(&true);
        let names_client = words.iter().any(|word| word.trim_end_matches('{') == name);
        [starts_impl, names_client] == [true, true]
    }

    fn failure(&self, at: usize, message: String, role: ContractRole) -> Failure {
        let portable = self.path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(role.help.to_owned()),
            source_span(&portable, at, (at + 1).min(self.source.len())),
        ))
    }
}
