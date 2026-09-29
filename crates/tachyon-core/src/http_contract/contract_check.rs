use super::client_declaration::ClientDeclaration;
use super::contract_form::ContractForm;
use super::contract_member::ContractMember;
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

type Problem = (usize, String);

impl<'a> ContractCheck<'a> {
    const CODE: u16 = 1308;
    const MARKER: &'static str = "HTTPClient";
    const NO_IMPLEMENTATION: &'static str = "An HTTPClient has no implementation: Tachyon generates each request. Move the logic into a service.";
    const REQUEST_MARKERS: [&'static str; 8] = [
        "GetRequest",
        "PostRequest",
        "PutRequest",
        "PatchRequest",
        "DeleteRequest",
        "StreamRequest",
        "SocketRequest",
        "Gateway",
    ];
    const TYPE_ONLY_FIELD: &'static str = "declare ";
    const TYPESCRIPT: &'static str = "ts";
    const GO: &'static str = "go";
    const RUST: &'static str = "rs";
    const GO_METHOD: &'static str = "func (";
    const RUST_IMPL: &'static str = "impl";
    const RUST_GENERIC_IMPL: &'static str = "impl<";
    const HELP: &'static str = "An HTTPClient is a contract, like a Spring @HttpExchange interface: each method is a signature that says which request it sends, and Tachyon writes the body, the constructor and the behavior context. `ty migrate fix --write` rewrites a class client. See docs/BEHAVIOR_ANNOTATIONS.md.";

    pub(crate) fn check(path: &'a Path, source: &'a str) -> Result<(), Failure> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        ContractForm::of(extension)
            .filter(|_| ClientDeclaration::is_mentioned(source))
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
        ClientDeclaration::find(self.form, self.source, &self.code)
            .iter()
            .find_map(|declaration| self.problem(declaration))
            .map_or(Ok(()), |(at, message)| Err(self.failure(at, message)))
    }

    fn problem(&self, declaration: &ClientDeclaration) -> Option<Problem> {
        self.role_problem(declaration)
            .or_else(|| self.form_problem(declaration))
            .or_else(|| self.member_problem(declaration))
            .or_else(|| self.implementation_problem(&declaration.name))
    }

    fn role_problem(&self, declaration: &ClientDeclaration) -> Option<Problem> {
        let spelled = Messages::spelled(self.form.language, Self::MARKER);
        let name = &declaration.name;
        declaration.other_role.as_ref().map(|role| {
            (
                declaration.marker_at,
                format!("{name} is marked with {spelled} and with the {role} role. An HTTPClient is its own role and has no implementation: keep the requests in {name}, and let a service that needs them declare a {name} field"),
            )
        })
    }

    fn form_problem(&self, declaration: &ClientDeclaration) -> Option<Problem> {
        let spelled = Messages::spelled(self.form.language, Self::MARKER);
        let header = &self.code[declaration.header.clone()];
        let message = format!(
            "{} is a {spelled}, so declare it as {}. {}",
            declaration.name,
            self.form.contract_words(),
            Self::NO_IMPLEMENTATION
        );
        (!self.form.declares_contract(header)).then_some((declaration.header.start, message))
    }

    fn member_problem(&self, declaration: &ClientDeclaration) -> Option<Problem> {
        declaration
            .members(self.form, &self.code)
            .iter()
            .find_map(|member| self.member_issue(&declaration.name, member))
    }

    fn member_issue(&self, name: &str, member: &ContractMember) -> Option<Problem> {
        self.constructor_issue(name, member)
            .or_else(|| self.state_issue(name, member))
            .or_else(|| self.body_issue(name, member))
            .or_else(|| self.annotation_issue(name, member))
    }

    fn constructor_issue(&self, name: &str, member: &ContractMember) -> Option<Problem> {
        let message = format!("{name} writes a constructor. {}", Self::NO_IMPLEMENTATION);
        member
            .is_constructor(&self.code)
            .then_some((member.start, message))
    }

    fn state_issue(&self, name: &str, member: &ContractMember) -> Option<Problem> {
        let text = self.code[member.start..member.end].trim();
        let type_only = [
            text.starts_with(Self::TYPE_ONLY_FIELD),
            self.form.language == Self::TYPESCRIPT,
        ] == [true, true];
        let message = format!(
            "{name} holds `{text}`. Tachyon creates the behavior context of a client itself, and a client keeps no other state. {}",
            Self::NO_IMPLEMENTATION
        );
        ([member.method, type_only] == [false, false]).then_some((member.start, message))
    }

    fn body_issue(&self, name: &str, member: &ContractMember) -> Option<Problem> {
        let braced = [self.form.has_contract(), self.form.is_indented()] == [true, false];
        let written = [member.has_code(&self.code), member.has_block()][usize::from(braced)];
        let message = format!(
            "{name}.{} has a body. {}",
            member.name(&self.code),
            Self::NO_IMPLEMENTATION
        );
        ([member.method, written] == [true, true]).then_some((member.start, message))
    }

    fn annotation_issue(&self, name: &str, member: &ContractMember) -> Option<Problem> {
        let message = format!(
            "{name}.{} has no request annotation. Each method of an HTTPClient sends a request: mark it with {}",
            member.name(&self.code),
            self.request_markers()
        );
        ([member.method, member.is_declared_call()] == [true, false])
            .then_some((member.start, message))
    }

    fn request_markers(&self) -> String {
        let spelled: Vec<String> = Self::REQUEST_MARKERS
            .iter()
            .map(|marker| Messages::spelled(self.form.language, marker))
            .collect();
        spelled.join(", ")
    }

    fn implementation_problem(&self, name: &str) -> Option<Problem> {
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
                        Self::NO_IMPLEMENTATION
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

    fn failure(&self, at: usize, message: String) -> Failure {
        let portable = self.path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(Self::HELP.to_owned()),
            source_span(&portable, at, (at + 1).min(self.source.len())),
        ))
    }
}
