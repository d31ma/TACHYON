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
    const HELP: &'static str = "An HTTPClient is a contract, like a Spring @HttpExchange interface: each method is a signature that says which request it sends, and Tachyon writes the body, the constructor and the behavior context. `ty migrate fix --write` rewrites a class client. See docs/BEHAVIOR_ANNOTATIONS.md.";

    pub(crate) fn check(path: &'a Path, source: &'a str) -> Result<(), Failure> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let Some(form) = ContractForm::of(extension) else {
            return Ok(());
        };
        if !ClientDeclaration::is_mentioned(source) {
            return Ok(());
        }
        let mask = crate::lexical::code_mask(extension, source);
        let checker = Self {
            path,
            source,
            code: HttpClientSource::masked(source, &mask.code),
            form,
        };
        for declaration in ClientDeclaration::find(form, source, &checker.code) {
            checker.declaration(&declaration)?;
        }
        Ok(())
    }

    fn declaration(&self, declaration: &ClientDeclaration) -> Result<(), Failure> {
        let spelled = Messages::spelled(self.form.language, Self::MARKER);
        let name = &declaration.name;
        if let Some(role) = &declaration.other_role {
            return Err(self.failure(
                declaration.marker_at,
                format!("{name} is marked with {spelled} and with the {role} role. An HTTPClient is its own role and has no implementation: keep the requests in {name}, and let a service that needs them declare a {name} field"),
            ));
        }
        let header = &self.code[declaration.header.clone()];
        if !self.form.declares_contract(header) {
            return Err(self.failure(
                declaration.header.start,
                format!(
                    "{name} is a {spelled}, so declare it as {}. {}",
                    self.form.contract_words(),
                    Self::NO_IMPLEMENTATION
                ),
            ));
        }
        for member in declaration.members(self.form, &self.code) {
            self.member(name, &member)?;
        }
        self.outside_implementation(name)
    }

    fn member(&self, name: &str, member: &ContractMember) -> Result<(), Failure> {
        let text = self.code[member.start..member.end].trim();
        let method = member.name(&self.code);
        if member.is_constructor(&self.code) {
            return Err(self.failure(
                member.start,
                format!("{name} writes a constructor. {}", Self::NO_IMPLEMENTATION),
            ));
        }
        if !member.method {
            if text.starts_with(Self::TYPE_ONLY_FIELD) && self.form.language == "ts" {
                return Ok(());
            }
            return Err(self.failure(
                member.start,
                format!("{name} holds `{text}`. Tachyon creates the behavior context of a client itself, and a client keeps no other state. {}", Self::NO_IMPLEMENTATION),
            ));
        }
        let written_body = if self.form.has_contract() && !self.form.is_indented() {
            member.has_block()
        } else {
            member.has_code(&self.code)
        };
        if written_body {
            return Err(self.failure(
                member.start,
                format!("{name}.{method} has a body. {}", Self::NO_IMPLEMENTATION),
            ));
        }
        if !member.is_declared_call() {
            return Err(self.failure(
                member.start,
                format!(
                    "{name}.{method} has no request annotation. Each method of an HTTPClient sends a request: mark it with {}",
                    self.request_markers()
                ),
            ));
        }
        Ok(())
    }

    fn request_markers(&self) -> String {
        let spelled: Vec<String> = Self::REQUEST_MARKERS
            .iter()
            .map(|marker| Messages::spelled(self.form.language, marker))
            .collect();
        spelled.join(", ")
    }

    fn outside_implementation(&self, name: &str) -> Result<(), Failure> {
        let mut offset = 0;
        for line in self.code.split_inclusive('\n') {
            let start = offset;
            offset += line.len();
            if self.implements(line.trim(), name) {
                return Err(self.failure(
                    start,
                    format!(
                        "This file implements {name} by hand. {}",
                        Self::NO_IMPLEMENTATION
                    ),
                ));
            }
        }
        Ok(())
    }

    fn implements(&self, line: &str, name: &str) -> bool {
        match self.form.language {
            "go" => line
                .strip_prefix("func (")
                .and_then(|rest| rest.split(')').next())
                .and_then(|receiver| receiver.split_whitespace().last())
                .is_some_and(|receiver| receiver.trim_start_matches('*') == name),
            "rs" => {
                let words: Vec<&str> = line.split_whitespace().collect();
                let starts_impl = words
                    .first()
                    .is_some_and(|word| *word == "impl" || word.starts_with("impl<"));
                starts_impl && words.iter().any(|word| word.trim_end_matches('{') == name)
            }
            _ => false,
        }
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
