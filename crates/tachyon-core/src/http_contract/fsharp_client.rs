use crate::behavior::HttpClientSource;
use serde_json::Value;
use std::path::Path;

pub(crate) struct FsharpClient<'a> {
    lines: Vec<&'a str>,
    code: Vec<String>,
}

struct FsharpDeclaration {
    header: usize,
    name: String,
    options: Value,
    body: Vec<usize>,
}

impl<'a> FsharpClient<'a> {
    const EXTENSION: &'static str = "fs";
    const ATTRIBUTE_OPEN: &'static str = "[<";
    const ATTRIBUTE_CLOSE: &'static str = ">]";
    const CLIENT: &'static str = "httpclient";
    const TYPE_WORD: &'static str = "type ";
    const CONSTRUCTOR: &'static str = "()";
    const CLIENT_SUFFIX: &'static str = "Client";
    const SETUP_SUFFIX: &'static str = "Setup";
    const DEFAULT_INDENT: &'static str = "    ";
    const SETUP_KEY: &'static str = "setup";
    const ADAPTER_KEY: &'static str = "adapter";
    const REQUESTS: [&'static str; 7] = [
        "getrequest",
        "postrequest",
        "putrequest",
        "patchrequest",
        "deleterequest",
        "streamrequest",
        "socketrequest",
    ];
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
            options: Self::options(self.lines[marker]),
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
            .find(|character: char| !(character.is_alphanumeric() || character == '_'))
            .unwrap_or(rest.len());
        let interface = rest[length..].trim_start().starts_with('=');
        ([length > 0, interface] == [true, true]).then(|| rest[..length].to_owned())
    }

    fn options(marker: &str) -> Value {
        let open = marker.find('(');
        let close = marker.rfind(')');
        open.zip(close)
            .filter(|(open, close)| close > open)
            .and_then(|(open, close)| serde_json::from_str(marker[open + 1..close].trim()).ok())
            .unwrap_or(Value::Null)
    }

    fn lowered(&self, declarations: &[FsharpDeclaration]) -> String {
        let mut output = String::new();
        for (index, line) in self.lines.iter().enumerate() {
            let declaration = declarations.iter().find(|found| found.header == index);
            match declaration {
                Some(declaration) => output.push_str(&self.concrete_header(declaration, line)),
                None => output.push_str(line),
            }
        }
        output
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
            .replace("{setup}", &Self::setup(declaration))
            .replace("{partition}", &Self::partition(declaration));
        format!("{}{text}\n\n", self.indent(declaration))
    }

    fn sends_request(line: &str) -> bool {
        let trimmed = line.trim_start();
        let marker = trimmed.starts_with(Self::ATTRIBUTE_OPEN);
        let names = Self::attribute_names(trimmed);
        let request = names
            .iter()
            .any(|name| Self::REQUESTS.contains(&name.as_str()));
        [marker, request] == [true, true]
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

    fn option(declaration: &FsharpDeclaration, key: &str) -> Option<String> {
        declaration
            .options
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    fn setup(declaration: &FsharpDeclaration) -> String {
        let name = &declaration.name;
        let stem = name.strip_suffix(Self::CLIENT_SUFFIX).unwrap_or(name);
        Self::option(declaration, Self::SETUP_KEY)
            .unwrap_or_else(|| format!("{stem}{}", Self::SETUP_SUFFIX))
    }

    fn partition(declaration: &FsharpDeclaration) -> String {
        Self::option(declaration, Self::ADAPTER_KEY)
            .unwrap_or_else(|| HttpClientSource::adapter_name(&declaration.name))
    }
}
