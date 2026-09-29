use super::client_declaration::ClientDeclaration;
use super::contract_form::ContractForm;
use super::contract_member::ContractMember;
use crate::behavior::HttpClientSource;
use std::ops::Range;
use std::path::Path;

pub(crate) struct ContractRewrite<'a> {
    form: ContractForm,
    source: &'a str,
    code: String,
}

pub(crate) type SourceEdit = (Range<usize>, String);

impl<'a> ContractRewrite<'a> {
    const CONTEXT_FIELD: &'static str = "behaviors";
    const START_MARKER: &'static str = "onstart";
    const RUST_CONSTRUCTOR: &'static str = "new";
    const DROPPED_MODIFIERS: [&'static str; 6] =
        ["final", "sealed", "abstract", "open", "data", "static"];
    const CSHARP_METHOD_MODIFIERS: [&'static str; 3] = ["public", "async", "virtual"];
    const PROTOCOL_IMPORT: &'static str = "from typing import Protocol\n";
    const TYPING_IMPORT: &'static str = "from typing import ";

    pub(crate) fn edits(path: &Path, source: &'a str) -> Result<Vec<SourceEdit>, String> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let Some(form) = ContractForm::of(extension).filter(|form| form.has_contract()) else {
            return Ok(Vec::new());
        };
        if !ClientDeclaration::is_mentioned(source) {
            return Ok(Vec::new());
        }
        let mask = crate::lexical::code_mask(extension, source);
        let rewrite = Self {
            form,
            source,
            code: HttpClientSource::masked(source, &mask.code),
        };
        let mut edits = Vec::new();
        for declaration in ClientDeclaration::find(form, source, &rewrite.code) {
            if form.declares_contract(&rewrite.code[declaration.header.clone()]) {
                continue;
            }
            if let Some(role) = &declaration.other_role {
                return Err(format!(
                    "{} is an HTTPClient and a {role}. Move the {role} code into its own class, then run the fix again.",
                    declaration.name
                ));
            }
            rewrite.declaration(&declaration, &mut edits)?;
        }
        Ok(edits)
    }

    fn declaration(
        &self,
        declaration: &ClientDeclaration,
        edits: &mut Vec<SourceEdit>,
    ) -> Result<(), String> {
        match self.form.language {
            "rs" => self.rust(declaration, edits),
            "go" => self.go(declaration, edits),
            _ => {
                edits.push((declaration.header.clone(), self.header(declaration)));
                let members = declaration.members(self.form, &self.code);
                for member in &members {
                    self.member(declaration, member, edits)?;
                }
                if self.form.language == "py" {
                    edits.push(self.protocol_import());
                }
                Ok(())
            }
        }
    }

    fn header(&self, declaration: &ClientDeclaration) -> String {
        let text = &self.source[declaration.header.clone()];
        if self.form.language == "py" {
            let open = text.find(['(', ':']).unwrap_or(text.len());
            return format!("{}(Protocol):", &text[..open]);
        }
        let kept: Vec<&str> = text
            .split_whitespace()
            .filter(|word| !Self::DROPPED_MODIFIERS.contains(word))
            .map(|word| match (word, self.form.language) {
                ("class", "swift") => "protocol",
                ("class", "dart") => "abstract interface class",
                ("class", _) => "interface",
                _ => word,
            })
            .collect();
        format!("{} ", kept.join(" "))
    }

    fn protocol_import(&self) -> SourceEdit {
        let mut offset = 0;
        for line in self.source.split_inclusive('\n') {
            if let Some(names) = line.trim_end().strip_prefix(Self::TYPING_IMPORT) {
                let end = offset + line.trim_end().len();
                if names.split(',').any(|name| name.trim() == "Protocol") {
                    return (end..end, String::new());
                }
                return (end..end, String::from(", Protocol"));
            }
            offset += line.len();
        }
        (0..0, format!("{}\n", Self::PROTOCOL_IMPORT))
    }

    fn is_context_scaffolding(&self, member: &ContractMember) -> bool {
        let name = member.name(&self.code);
        let starts = member
            .markers
            .iter()
            .any(|marker| marker == Self::START_MARKER);
        (!member.method && name == Self::CONTEXT_FIELD)
            || starts
            || (member.method && name == Self::RUST_CONSTRUCTOR)
    }

    fn removal(&self, member: &ContractMember) -> SourceEdit {
        let start = member.first_marker.unwrap_or(member.start);
        let start = self.code[..start].rfind('\n').map_or(0, |at| at + 1);
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

    fn member(
        &self,
        declaration: &ClientDeclaration,
        member: &ContractMember,
        edits: &mut Vec<SourceEdit>,
    ) -> Result<(), String> {
        if self.is_context_scaffolding(member) {
            edits.push(self.removal(member));
            return Ok(());
        }
        let name = member.name(&self.code);
        if !member.method || member.has_code(&self.code) || !member.is_declared_call() {
            return Err(format!(
                "{}.{name} holds code or state. An HTTPClient has no implementation: move it into a service, then run the fix again.",
                declaration.name
            ));
        }
        edits.push(self.bodiless(member));
        Ok(())
    }

    fn bodiless(&self, member: &ContractMember) -> SourceEdit {
        let end = member.body.as_ref().map_or(member.end, |body| {
            if self.form.language == "py" {
                body.end
            } else {
                body.end + 1
            }
        });
        let signature = &self.source[member.start..member.end];
        let replacement = match self.form.language {
            "py" => {
                let body = &self.source[member.end..end];
                let blank_lines = body.len() - body.trim_end_matches(['\n', ' ']).len();
                let newlines = body[body.len() - blank_lines..].matches('\n').count();
                return (member.end..end, format!(": ...{}", "\n".repeat(newlines)));
            }
            "cs" => format!(
                "{};",
                Self::without_words(signature, &Self::CSHARP_METHOD_MODIFIERS)
            ),
            "dart" => format!(
                "{};",
                signature.trim_end().trim_end_matches("async").trim_end()
            ),
            "java" | "php" => format!("{};", signature.trim_end()),
            _ => signature.trim_end().to_owned(),
        };
        (member.start..end, replacement)
    }

    fn without_words(signature: &str, words: &[&str]) -> String {
        let mut rest = signature.trim_start();
        while let Some(word) = rest.split_whitespace().next() {
            if !words.contains(&word) {
                break;
            }
            rest = rest[word.len()..].trim_start();
        }
        rest.trim_end().to_owned()
    }

    fn rust(
        &self,
        declaration: &ClientDeclaration,
        edits: &mut Vec<SourceEdit>,
    ) -> Result<(), String> {
        let name = &declaration.name;
        let after_close = (declaration.close + 1).min(self.source.len());
        let struct_end = if self.code.as_bytes().get(declaration.close) == Some(&b'}') {
            self.code[after_close..]
                .find('\n')
                .map_or(self.source.len(), |at| after_close + at + 1)
        } else {
            after_close
        };
        let Some((impl_header, impl_body)) = self.rust_impl(name) else {
            return Err(format!(
                "{name} has no `impl {name}` block. Write it as a trait by hand."
            ));
        };
        let state = self.code[declaration.body.clone()]
            .split(',')
            .map(str::trim)
            .any(|field| !field.is_empty() && !field.starts_with(Self::CONTEXT_FIELD));
        if state {
            return Err(format!(
                "{name} holds fields. An HTTPClient has no implementation: move them into a service, then run the fix again."
            ));
        }
        let prefix = self.source[declaration.header.clone()]
            .split("struct")
            .next()
            .unwrap_or("")
            .to_owned();
        edits.push((declaration.header.start..struct_end, String::new()));
        edits.push((impl_header, format!("{prefix}trait {name} ")));
        let form = self.form;
        let body = ClientDeclaration {
            body: impl_body,
            ..declaration.clone()
        };
        for member in body.members(form, &self.code) {
            if self.is_context_scaffolding(&member) {
                edits.push(self.removal(&member));
                continue;
            }
            self.member(declaration, &member, edits)?;
            if let Some(last) = edits.last_mut() {
                last.1.push(';');
            }
        }
        Ok(())
    }

    fn rust_impl(&self, name: &str) -> Option<(Range<usize>, Range<usize>)> {
        let mut offset = 0;
        for line in self.code.split_inclusive('\n') {
            let start = offset;
            offset += line.len();
            let words: Vec<&str> = line.split_whitespace().collect();
            let names_type = words
                .get(1)
                .is_some_and(|word| word.trim_end_matches('{') == name);
            if words.first() != Some(&"impl") || !names_type || words.contains(&"for") {
                continue;
            }
            let open = start + line.find('{')?;
            let close = ClientDeclaration::matching_brace(&self.code, open)?;
            return Some((start..open, open + 1..close));
        }
        None
    }

    fn go(
        &self,
        declaration: &ClientDeclaration,
        edits: &mut Vec<SourceEdit>,
    ) -> Result<(), String> {
        let name = &declaration.name;
        let fields: Vec<&str> = self.code[declaration.body.clone()]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with(Self::CONTEXT_FIELD))
            .collect();
        if !fields.is_empty() {
            return Err(format!(
                "{name} holds fields. An HTTPClient has no implementation: move them into a service, then run the fix again."
            ));
        }
        let mut signatures = Vec::new();
        for method in self.go_methods(name) {
            edits.push((method.whole.clone(), String::new()));
            if method.starts_context {
                continue;
            }
            if !self.code[method.body.clone()].trim().is_empty() {
                return Err(format!(
                    "{name}.{} has a body. An HTTPClient has no implementation: move it into a service, then run the fix again.",
                    method.signature.split('(').next().unwrap_or("")
                ));
            }
            signatures.push(method.declaration());
        }
        let end = (declaration.close + 1).min(self.source.len());
        edits.push((
            declaration.header.start..end,
            format!("type {name} interface {{\n{}}}", signatures.join("\n")),
        ));
        Ok(())
    }

    fn go_methods(&self, name: &str) -> Vec<GoMethod> {
        let mut methods = Vec::new();
        let mut offset = 0;
        let mut markers: Vec<(usize, String)> = Vec::new();
        for line in self.code.split_inclusive('\n') {
            let start = offset;
            offset += line.len();
            let original = self.source[start..offset].trim();
            if original.starts_with("// @") {
                markers.push((start, original.to_owned()));
                continue;
            }
            let receiver = line
                .strip_prefix("func (")
                .and_then(|rest| rest.split(')').next())
                .and_then(|receiver| receiver.split_whitespace().last())
                .map(|receiver| receiver.trim_start_matches('*'));
            if receiver == Some(name)
                && let Some(method) = self.go_method(start, &markers)
            {
                methods.push(method);
            }
            if !line.trim().is_empty() {
                markers.clear();
            }
        }
        methods
    }

    fn go_method(&self, start: usize, markers: &[(usize, String)]) -> Option<GoMethod> {
        let receiver_close = start + self.code[start..].find(')')?;
        let mut depth = 0_i32;
        let mut open = None;
        for (at, byte) in self.code.bytes().enumerate().skip(receiver_close + 1) {
            match byte {
                b'(' => depth += 1,
                b')' => depth -= 1,
                b'{' if depth == 0 => {
                    open = Some(at);
                    break;
                }
                _ => {}
            }
        }
        let open = open?;
        let close = ClientDeclaration::matching_brace(&self.code, open)?;
        let whole_start = markers.first().map_or(start, |(at, _)| *at);
        let whole_end = self.code[close..]
            .find('\n')
            .map_or(self.source.len(), |at| close + at + 1);
        Some(GoMethod {
            markers: markers.iter().map(|(_, text)| text.clone()).collect(),
            signature: self.source[receiver_close + 1..open].trim().to_owned(),
            body: open + 1..close,
            whole: whole_start..whole_end,
            starts_context: markers.iter().any(|(_, text)| text == "// @OnStart"),
        })
    }
}

struct GoMethod {
    markers: Vec<String>,
    signature: String,
    body: Range<usize>,
    whole: Range<usize>,
    starts_context: bool,
}

impl GoMethod {
    fn declaration(&self) -> String {
        let mut text = String::new();
        for marker in &self.markers {
            text.push('\t');
            text.push_str(marker);
            text.push('\n');
        }
        for line in self.signature.lines() {
            text.push('\t');
            text.push_str(line.trim_start_matches('\t'));
            text.push('\n');
        }
        text
    }
}
