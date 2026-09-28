use super::ManualStep;
use super::edits::TextEdit;
use super::markers::{Marker, MarkerLine};
use super::python_parameters::PythonParameterEdits;
use super::scan::SourceText;
use crate::behavior::HttpClientSource;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) struct ClientConversion<'a> {
    relative: &'a str,
    extension: &'a str,
    text: SourceText<'a>,
}

#[derive(Default)]
struct Owner {
    adapter: Option<String>,
    marker: Option<MarkerLine>,
    cache: Option<Value>,
}

impl<'a> ClientConversion<'a> {
    const CALL_KEYS: [&'static str; 6] = ["client", "method", "path", "query", "headers", "body"];
    const VERBS: [(&'static str, &'static str); 5] = [
        ("GET", "GetRequest"),
        ("POST", "PostRequest"),
        ("PUT", "PutRequest"),
        ("PATCH", "PatchRequest"),
        ("DELETE", "DeleteRequest"),
    ];

    pub(super) fn plan(
        relative: &'a str,
        source: &'a str,
    ) -> Result<(Vec<TextEdit>, Vec<String>), ManualStep> {
        let extension = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let Some(text) = SourceText::read(extension, source) else {
            return Ok((Vec::new(), Vec::new()));
        };
        let conversion = Self {
            relative,
            extension,
            text,
        };
        conversion.convert()
    }

    fn convert(&self) -> Result<(Vec<TextEdit>, Vec<String>), ManualStep> {
        let markers = Marker::lines(&self.text, self.extension, &["client", "webcall"]);
        let mut edits = Vec::new();
        let mut notes = Vec::new();
        let mut owners: BTreeMap<String, Owner> = BTreeMap::new();
        for marker in &markers {
            let line = self.text.line_number(marker.start);
            let Some(declaration) =
                Marker::next_declaration(&self.text, self.extension, marker.line_start)
            else {
                return Err(self.manual(line, "Nothing follows this marker."));
            };
            if Marker::normalized(&marker.name) == "client" {
                let name = self
                    .declared_name(declaration)
                    .ok_or_else(|| self.manual(line, "@Client is not followed by a class."))?;
                let owner = owners.entry(name).or_default();
                owner.cache = self.client_options(marker, line)?;
                owner.marker = Some(marker.clone());
                continue;
            }
            let (adapter, replacement, parameters) = self.call(marker, declaration, line)?;
            let owner_name = self.owner(declaration).ok_or_else(|| {
                self.manual(line, "The fix cannot find the class of this method.")
            })?;
            let owner = owners.entry(owner_name).or_default();
            if owner
                .adapter
                .as_ref()
                .is_some_and(|known| *known != adapter)
            {
                return Err(self.manual(
                    line,
                    "The methods of this class use more than one WebCall client name.",
                ));
            }
            owner.adapter = Some(adapter);
            notes.push(format!(
                "Changed @WebCall to {replacement} on line {line} (ADR 0048)."
            ));
            edits.push(TextEdit::replace(marker.start, marker.end, &replacement));
            edits.extend(parameters);
        }
        for (name, owner) in &owners {
            self.owner_marker(name, owner, &mut edits, &mut notes)?;
        }
        Ok((edits, notes))
    }

    fn owner_marker(
        &self,
        name: &str,
        owner: &Owner,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> Result<(), ManualStep> {
        let mut options = Map::new();
        let default = HttpClientSource::adapter_name(name);
        if let Some(adapter) = owner
            .adapter
            .as_ref()
            .filter(|adapter| **adapter != default)
        {
            options.insert(String::from("adapter"), Value::String(adapter.clone()));
        }
        if let Some(cache) = &owner.cache {
            options.insert(String::from("cache"), cache.clone());
        }
        let arguments = if options.is_empty() {
            String::new()
        } else {
            Value::Object(options).to_string()
        };
        let marker = Marker::spelled(self.extension, "HTTPClient", &arguments);
        if let Some(old) = &owner.marker {
            notes.push(format!(
                "Changed @Client to {marker} on line {} (ADR 0048).",
                self.text.line_number(old.start)
            ));
            edits.push(TextEdit::replace(old.start, old.end, &marker));
            return Ok(());
        }
        let Some(declaration) = self.declaration_of(name) else {
            return Err(self.manual(1, "The fix cannot find the class that sends the requests."));
        };
        let line_start = self.text.line_start(declaration);
        let indentation = Marker::indentation(&self.text, line_start);
        notes.push(format!(
            "Added {marker} to the class {name}, because it sends requests (ADR 0048)."
        ));
        edits.push(TextEdit::insert(
            line_start,
            &format!("{indentation}{marker}\n"),
        ));
        Ok(())
    }

    fn client_options(
        &self,
        marker: &MarkerLine,
        line: usize,
    ) -> Result<Option<Value>, ManualStep> {
        let Some((start, end)) = marker.arguments else {
            return Ok(None);
        };
        let written = self.text.source[start..end].trim();
        if written.is_empty() {
            return Ok(None);
        }
        let Ok(Value::Object(object)) = serde_json::from_str::<Value>(written) else {
            return Err(self.manual(line, "The fix cannot read the options of @Client."));
        };
        if object.keys().any(|key| key != "cache") {
            return Err(self.manual(line, "@Client has an option other than \"cache\"."));
        }
        Ok(object.get("cache").cloned())
    }

    fn call(
        &self,
        marker: &MarkerLine,
        declaration: usize,
        line: usize,
    ) -> Result<(String, String, Vec<TextEdit>), ManualStep> {
        let object = marker
            .arguments
            .and_then(|(start, end)| {
                serde_json::from_str::<Value>(self.text.source[start..end].trim()).ok()
            })
            .and_then(|value| value.as_object().cloned())
            .ok_or_else(|| self.manual(line, "The fix cannot read the object in @WebCall."))?;
        if object
            .keys()
            .any(|key| !Self::CALL_KEYS.contains(&key.as_str()))
        {
            return Err(self.manual(line, "@WebCall has a key the fix does not know."));
        }
        let adapter = object.get("client").and_then(Value::as_str);
        let path = object.get("path").and_then(Value::as_str);
        let verb = object
            .get("method")
            .and_then(Value::as_str)
            .and_then(|method| Self::VERBS.iter().find(|(verb, _)| *verb == method));
        let (Some(adapter), Some(path), Some((_, request))) = (adapter, path, verb) else {
            return Err(self.manual(
                line,
                "@WebCall needs a client, a path and a GET, POST, PUT, PATCH or DELETE method.",
            ));
        };
        let replacement = Marker::spelled(self.extension, request, &Marker::quoted(path));
        let parameters = self.parameter_markers(&object, declaration, line)?;
        Ok((String::from(adapter), replacement, parameters))
    }

    fn parameter_markers(
        &self,
        object: &Map<String, Value>,
        declaration: usize,
        line: usize,
    ) -> Result<Vec<TextEdit>, ManualStep> {
        let mut wanted: Vec<(u64, String)> = Vec::new();
        for (key, marker) in [("query", "RequestParam"), ("headers", "RequestHeader")] {
            let Some(map) = object.get(key) else {
                continue;
            };
            let map = map
                .as_object()
                .ok_or_else(|| self.manual(line, "The fix cannot read the @WebCall mappings."))?;
            for (name, index) in map {
                let index = index.as_u64().ok_or_else(|| {
                    self.manual(line, "The fix cannot read the @WebCall mappings.")
                })?;
                let written = Marker::spelled(self.extension, marker, &Marker::quoted(name));
                wanted.push((index, written));
            }
        }
        if let Some(body) = object.get("body") {
            let index = body
                .as_u64()
                .ok_or_else(|| self.manual(line, "The fix cannot read the @WebCall body index."))?;
            wanted.push((index, Marker::spelled(self.extension, "RequestBody", "")));
        }
        if wanted.is_empty() {
            return Ok(Vec::new());
        }
        let starts = self.parameter_starts(declaration).ok_or_else(|| {
            self.manual(line, "The fix cannot read the parameters of this method.")
        })?;
        let mut edits = Vec::new();
        let mut annotated = false;
        for (index, marker) in wanted {
            let at = usize::try_from(index)
                .ok()
                .and_then(|index| starts.get(index))
                .ok_or_else(|| {
                    self.manual(line, "@WebCall names a parameter the method does not have.")
                })?;
            if self.extension == "py" {
                let marker = marker.trim_start_matches('@');
                let edit =
                    PythonParameterEdits::wrap(&self.text, *at, *at, marker).ok_or_else(|| {
                        self.manual(
                            line,
                            "The fix cannot move this marker into the parameter's type.",
                        )
                    })?;
                edits.push(edit);
                annotated = true;
                continue;
            }
            let written = if self.extension == "go" {
                format!("\n{marker}\n")
            } else {
                format!("{marker} ")
            };
            edits.push(TextEdit::insert(*at, &written));
        }
        if annotated && let Some(import) = PythonParameterEdits::import(self.text.source) {
            edits.push(import);
        }
        Ok(edits)
    }

    fn parameter_starts(&self, declaration: usize) -> Option<Vec<usize>> {
        let mut search = declaration;
        if self.extension == "rs" {
            search = declaration + self.text.source[declaration..].find("fn ")?;
        }
        let mut open = self.next_open(search)?;
        if self.extension == "go" {
            open = self.next_open(self.text.matching(open)? + 1)?;
        }
        let close = self.text.matching(open)?;
        let mut starts = Vec::new();
        let mut piece = open + 1;
        let mut depth = 0_i32;
        for at in open + 1..=close {
            if !self.text.is_code(at) {
                continue;
            }
            match self.text.bytes[at] {
                b'(' | b'[' | b'{' | b'<' => depth += 1,
                b')' | b']' | b'}' | b'>' if at != close => depth -= 1,
                b',' if depth == 0 => {
                    self.push_parameter(piece, at, &mut starts);
                    piece = at + 1;
                }
                _ => {}
            }
        }
        self.push_parameter(piece, close, &mut starts);
        Some(starts)
    }

    fn push_parameter(&self, start: usize, end: usize, starts: &mut Vec<usize>) {
        let Some(first) = self.text.next_code(start).filter(|first| *first < end) else {
            return;
        };
        let written = self.text.source[first..end].trim();
        let is_receiver = match self.extension {
            "py" => written == "self",
            "rs" => matches!(written, "&self" | "&mut self" | "self" | "mut self"),
            _ => false,
        };
        if !is_receiver {
            starts.push(first);
        }
    }

    fn next_open(&self, from: usize) -> Option<usize> {
        (from..self.text.len()).find(|at| self.text.is_code(*at) && self.text.bytes[*at] == b'(')
    }

    fn declared_name(&self, declaration: usize) -> Option<String> {
        let line = self.text.line_text(self.text.line_start(declaration));
        let words: Vec<&str> = line
            .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            .filter(|word| !word.is_empty())
            .collect();
        words
            .windows(2)
            .find(|pair| self.declaration_words().contains(&pair[0]))
            .map(|pair| String::from(pair[1]))
    }

    fn declaration_words(&self) -> &'static [&'static str] {
        match self.extension {
            "go" => &["type"],
            "rs" => &["struct"],
            "kt" => &["class", "object", "interface"],
            "swift" => &["class", "struct"],
            "cs" => &["class", "struct", "record", "interface"],
            _ => &["class", "interface"],
        }
    }

    fn owner(&self, declaration: usize) -> Option<String> {
        let line = self.text.line_text(self.text.line_start(declaration));
        if self.extension == "go" {
            let receiver = line.trim_start().strip_prefix("func (")?;
            let receiver = receiver.split(')').next()?;
            let name = receiver.split_whitespace().last()?.trim_start_matches('*');
            return Some(String::from(name));
        }
        let mut line_start = self.text.line_start(declaration);
        while let Some(previous) = self.text.previous_line_start(line_start) {
            line_start = previous;
            let at = line_start + self.text.indentation(line_start);
            if !self.text.is_code(at) {
                continue;
            }
            let trimmed = self.text.line_text(line_start).trim_start();
            if self.extension == "rs" {
                if let Some(rest) = trimmed.strip_prefix("impl") {
                    let name = rest
                        .trim_start_matches(|character: char| !character.is_ascii_alphabetic())
                        .split(|character: char| {
                            !(character.is_ascii_alphanumeric() || character == '_')
                        })
                        .next()?;
                    return Some(String::from(name));
                }
                continue;
            }
            if let Some(name) = self.declared_name(at) {
                return Some(name);
            }
        }
        None
    }

    fn declaration_of(&self, name: &str) -> Option<usize> {
        let mut line_start = 0;
        while line_start < self.text.len() {
            let at = line_start + self.text.indentation(line_start);
            let is_rust_impl = self.extension == "rs"
                && self
                    .text
                    .line_text(line_start)
                    .trim_start()
                    .starts_with("impl");
            if self.text.is_code(at)
                && !is_rust_impl
                && self.declared_name(at).as_deref() == Some(name)
            {
                return Some(at);
            }
            line_start = self.text.line_end(line_start);
        }
        None
    }

    fn manual(&self, line: usize, problem: &str) -> ManualStep {
        ManualStep::new(
            &format!("{}:{line}", self.relative),
            format!("{problem} So the fix cannot convert @Client and @WebCall here (ADR 0048)."),
            String::from(
                "Mark the class @HTTPClient. Mark each method @GetRequest(\"/path\"), @PostRequest and so on. Mark its parameters @PathVariable, @RequestParam, @RequestBody or @RequestHeader.",
            ),
        )
    }
}
