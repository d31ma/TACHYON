use super::client_declaration::ClientDeclaration;
use super::context_members::ContextMembers;
use super::contract_form::ContractForm;
use super::contract_member::ContractMember;
use crate::behavior::HttpClientSource;
use std::fmt::Write as _;
use std::ops::Range;
use std::path::Path;

pub(crate) struct ConcreteClient<'a> {
    form: ContractForm,
    source: &'a str,
    code: String,
    tac: bool,
}

impl<'a> ConcreteClient<'a> {
    const TAC: &'static str = "tac";
    const EMPTY_BODY: &'static str = " {}";
    const DEFAULT_INDENT: &'static str = "    ";
    const LEADING_MODIFIERS: [&'static str; 4] = ["public", "abstract", "virtual", "async"];
    const GO_RECEIVER: &'static str = "client";

    pub(crate) fn lower(path: &Path, source: &'a str, environment: &str) -> Option<String> {
        let extension = path.extension().and_then(|value| value.to_str())?;
        let form = ContractForm::of(extension)?;
        let tac = environment == Self::TAC;
        let managed_browser_client = tac && matches!(form.language, "js" | "ts");
        if managed_browser_client
            || !ClientDeclaration::is_mentioned(source)
        {
            return None;
        }
        let mask = crate::lexical::code_mask(extension, source);
        let client = Self {
            form,
            source,
            code: HttpClientSource::masked(source, &mask.code),
            tac,
        };
        let mut edits = Vec::new();
        for declaration in ClientDeclaration::find(form, source, &client.code) {
            if form.declares_contract(&client.code[declaration.header.clone()]) {
                client.declaration_edits(&declaration, &mut edits);
            }
        }
        (!edits.is_empty()).then(|| Self::apply(source, edits))
    }

    fn apply(source: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
        edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
        let mut output = source.to_owned();
        for (range, replacement) in edits {
            output.replace_range(range, &replacement);
        }
        output
    }

    fn declaration_edits(
        &self,
        declaration: &ClientDeclaration,
        edits: &mut Vec<(Range<usize>, String)>,
    ) {
        let members = declaration.members(self.form, &self.code);
        let needs_context = members.iter().any(ContractMember::sends_request);
        if self.form.language == "go" {
            edits.push((
                declaration.header.start..(declaration.close + 1).min(self.source.len()),
                self.go_type(declaration, &members, needs_context),
            ));
            return;
        }
        if let Some(header) = self.header(declaration, needs_context) {
            edits.push((declaration.header.clone(), header));
        }
        for member in members
            .iter()
            .filter(|member| member.method && !member.has_block())
        {
            if let Some(edit) = self.method_edit(member) {
                edits.push(edit);
            }
        }
        let rust = self.form.language == "rs";
        if needs_context || rust {
            let indent = self.indent(&members);
            let setup = if needs_context {
                declaration.setup()
            } else {
                String::new()
            };
            let context = ContextMembers {
                language: self.form.language,
                tac: self.tac,
                client: &declaration.name,
                setup: &setup,
                partition: &declaration.adapter(),
                indent: &indent,
            };
            let (field, members) = context.parts();
            if !field.is_empty() {
                let open = declaration.header.end + 1;
                edits.push((open..open, field));
            }
            edits.push((declaration.close..declaration.close, members));
        }
    }

    fn indent(&self, members: &[ContractMember]) -> String {
        let Some(first) = members.first() else {
            return String::from(Self::DEFAULT_INDENT);
        };
        let at = first.start;
        let line_start = self.code[..at].rfind('\n').map_or(0, |line| line + 1);
        let prefix = &self.source[line_start..at.max(line_start)];
        let width = prefix.len() - prefix.trim_start().len();
        prefix[..width].to_owned()
    }

    fn header(&self, declaration: &ClientDeclaration, needs_context: bool) -> Option<String> {
        let text = &self.source[declaration.header.clone()];
        let name = &declaration.name;
        match self.form.language {
            "java" | "groovy" | "kt" | "cs" | "php" => {
                Self::replace_word(text, "interface", "class")
            }
            "dart" => Some(text.replacen("abstract interface class", "class", 1)),
            "swift" => Self::replace_word(text, "protocol", "final class"),
            "py" => Some(Self::without_protocol(text)),
            "rs" => {
                let fields = if needs_context {
                    " { behaviors: BehaviorContext }"
                } else {
                    ";"
                };
                let visibility = text.split("trait").next().unwrap_or("");
                Some(format!("{visibility}struct {name}{fields}\nimpl {name} "))
            }
            _ => None,
        }
    }

    fn replace_word(text: &str, word: &str, replacement: &str) -> Option<String> {
        let mut offset = 0;
        for piece in text.split_inclusive(char::is_whitespace) {
            if piece.trim_end() == word {
                let mut output = text[..offset].to_owned();
                output.push_str(replacement);
                output.push_str(&text[offset + word.len()..]);
                return Some(output);
            }
            offset += piece.len();
        }
        None
    }

    fn without_protocol(text: &str) -> String {
        let (Some(open), Some(close)) = (text.find('('), text.rfind(')')) else {
            return text.to_owned();
        };
        let bases: Vec<&str> = text[open + 1..close]
            .split(',')
            .map(str::trim)
            .filter(|base| !matches!(*base, "Protocol" | "typing.Protocol" | ""))
            .collect();
        let kept = if bases.is_empty() {
            String::new()
        } else {
            format!("({})", bases.join(", "))
        };
        format!("{}{kept}{}", &text[..open], &text[close + 1..])
    }

    fn method_edit(&self, member: &ContractMember) -> Option<(Range<usize>, String)> {
        let language = self.form.language;
        if matches!(language, "py" | "js" | "ts" | "rb") {
            return None;
        }
        let signature = &self.source[member.start..member.end];
        let end = member
            .terminator
            .map_or(member.end, |terminator| terminator + 1);
        let replacement = match language {
            "cs" => {
                let bare = Self::without_modifiers(signature);
                let asynchronous = if bare.starts_with("Task")
                    || bare.starts_with("System.Threading.Tasks.Task")
                {
                    "async "
                } else {
                    ""
                };
                format!("public {asynchronous}{bare}{}", Self::EMPTY_BODY)
            }
            "dart" => {
                let body = if signature.trim_start().starts_with("Future") {
                    " async {}"
                } else {
                    Self::EMPTY_BODY
                };
                format!("{signature}{body}")
            }
            "java" | "groovy" | "kt" => {
                format!("{}{}", Self::without_modifiers(signature), Self::EMPTY_BODY)
            }
            _ => format!("{signature}{}", Self::EMPTY_BODY),
        };
        Some((member.start..end, replacement))
    }

    fn without_modifiers(signature: &str) -> String {
        let mut rest = signature.trim_start();
        while let Some(word) = rest.split_whitespace().next() {
            if !Self::LEADING_MODIFIERS.contains(&word) {
                break;
            }
            rest = rest[word.len()..].trim_start();
        }
        rest.to_owned()
    }

    fn go_type(
        &self,
        declaration: &ClientDeclaration,
        members: &[ContractMember],
        needs_context: bool,
    ) -> String {
        let name = &declaration.name;
        let mut text = if needs_context {
            format!("type {name} struct {{\n\tbehaviors *BehaviorContext\n}}\n")
        } else {
            format!("type {name} struct{{}}\n")
        };
        for member in members.iter().filter(|member| member.method) {
            text.push('\n');
            if let Some(first) = member.first_marker {
                for line in self.source[first..member.start].lines() {
                    if !line.trim().is_empty() {
                        text.push_str(line.trim());
                        text.push('\n');
                    }
                }
            }
            let _ = write!(
                text,
                "func ({} *{name}) {} {{\n}}\n",
                Self::GO_RECEIVER,
                self.source[member.start..member.end].trim()
            );
        }
        if needs_context {
            let setup = declaration.setup();
            let context = ContextMembers {
                language: self.form.language,
                tac: self.tac,
                client: name,
                setup: &setup,
                partition: &declaration.adapter(),
                indent: "",
            };
            text.push_str(&context.go());
        }
        text
    }
}
