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
    const RUST_RECEIVERS: [&'static str; 4] = ["&self", "&mut self", "self", "mut self"];

    pub(super) fn plan(
        relative: &'a str,
        source: &'a str,
    ) -> Result<(Vec<TextEdit>, Vec<String>), ManualStep> {
        let extension = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let text = SourceText::read(extension, source);
        text.map_or_else(
            || Ok((Vec::new(), Vec::new())),
            |text| {
                let conversion = Self {
                    relative,
                    extension,
                    text,
                };
                conversion.convert()
            },
        )
    }

    fn convert(&self) -> Result<(Vec<TextEdit>, Vec<String>), ManualStep> {
        let markers = Marker::lines(&self.text, self.extension, &["client", "webcall"]);
        let mut edits = Vec::new();
        let mut notes = Vec::new();
        let mut owners: BTreeMap<String, Owner> = BTreeMap::new();
        for marker in &markers {
            self.read_marker(marker, &mut owners, &mut edits, &mut notes)?;
        }
        self.add_owner_markers(&owners, &mut edits, &mut notes)?;
        Ok((edits, notes))
    }

    fn read_marker(
        &self,
        marker: &MarkerLine,
        owners: &mut BTreeMap<String, Owner>,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> Result<(), ManualStep> {
        let line = self.text.line_number(marker.start);
        let declaration = Marker::next_declaration(&self.text, self.extension, marker.line_start)
            .ok_or_else(|| self.manual(line, "Nothing follows this marker."))?;
        if Marker::normalized(&marker.name) == "client" {
            return self.read_client(marker, declaration, line, owners);
        }
        self.read_web_call(marker, declaration, line, owners, edits, notes)
    }

    fn read_client(
        &self,
        marker: &MarkerLine,
        declaration: usize,
        line: usize,
        owners: &mut BTreeMap<String, Owner>,
    ) -> Result<(), ManualStep> {
        let name = self
            .declared_name(declaration)
            .ok_or_else(|| self.manual(line, "@Client is not followed by a class."))?;
        let owner = owners.entry(name).or_default();
        owner.cache = self.client_options(marker, line)?;
        owner.marker = Some(marker.clone());
        Ok(())
    }

    fn read_web_call(
        &self,
        marker: &MarkerLine,
        declaration: usize,
        line: usize,
        owners: &mut BTreeMap<String, Owner>,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> Result<(), ManualStep> {
        let (adapter, replacement, parameters) = self.call(marker, declaration, line)?;
        let owner_name = self
            .owner(declaration)
            .ok_or_else(|| self.manual(line, "The fix cannot find the class of this method."))?;
        let owner = owners.entry(owner_name).or_default();
        let conflicting = owner
            .adapter
            .as_ref()
            .is_some_and(|known| *known != adapter);
        (!conflicting).then_some(()).ok_or_else(|| {
            self.manual(
                line,
                "The methods of this class use more than one WebCall client name.",
            )
        })?;
        owner.adapter = Some(adapter);
        notes.push(format!(
            "Changed @WebCall to {replacement} on line {line} (ADR 0048)."
        ));
        edits.push(TextEdit::replace(marker.start, marker.end, &replacement));
        edits.extend(parameters);
        Ok(())
    }

    fn add_owner_markers(
        &self,
        owners: &BTreeMap<String, Owner>,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> Result<(), ManualStep> {
        for (name, owner) in owners {
            self.owner_marker(name, owner, edits, notes)?;
        }
        Ok(())
    }

    fn owner_marker(
        &self,
        name: &str,
        owner: &Owner,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> Result<(), ManualStep> {
        let marker = self.client_marker(name, owner);
        notes.extend(Self::cache_note(name, owner));
        let Some(old) = &owner.marker else {
            return self.add_client_marker(name, &marker, edits, notes);
        };
        notes.push(format!(
            "Changed @Client to {marker} on line {} (ADR 0048).",
            self.text.line_number(old.start)
        ));
        edits.push(TextEdit::replace(old.start, old.end, &marker));
        Ok(())
    }

    fn client_marker(&self, name: &str, owner: &Owner) -> String {
        let default = HttpClientSource::adapter_name(name);
        let adapter = owner
            .adapter
            .as_ref()
            .filter(|adapter| **adapter != default);
        let options: Map<String, Value> = adapter
            .map(|adapter| (String::from("adapter"), Value::String(adapter.clone())))
            .into_iter()
            .collect();
        let arguments = Self::option_text(options);
        Marker::spelled(self.extension, "HTTPClient", &arguments)
    }

    fn option_text(options: Map<String, Value>) -> String {
        if options.is_empty() {
            return String::new();
        }
        Value::Object(options).to_string()
    }

    fn cache_note(name: &str, owner: &Owner) -> Option<String> {
        let cache = owner
            .cache
            .as_ref()
            .filter(|cache| cache.as_str() != Some("no-store"));
        cache.map(|cache| {
            format!(
                "Dropped the {cache} cache option of @Client on {name}: mark the class @CacheSettings and each read method @Cached instead (ADR 0087)."
            )
        })
    }

    fn add_client_marker(
        &self,
        name: &str,
        marker: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> Result<(), ManualStep> {
        let declaration = self.declaration_of(name).ok_or_else(|| {
            self.manual(1, "The fix cannot find the class that sends the requests.")
        })?;
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
        let written = marker
            .arguments
            .map(|(start, end)| self.text.source[start..end].trim())
            .filter(|written| !written.is_empty());
        let Some(written) = written else {
            return Ok(None);
        };
        self.cache_option(written, line)
    }

    fn cache_option(&self, written: &str, line: usize) -> Result<Option<Value>, ManualStep> {
        let parsed = serde_json::from_str::<Value>(written).ok();
        let object = parsed
            .as_ref()
            .and_then(Value::as_object)
            .ok_or_else(|| self.manual(line, "The fix cannot read the options of @Client."))?;
        let only_cache = object.keys().all(|key| key == "cache");
        only_cache
            .then_some(())
            .ok_or_else(|| self.manual(line, "@Client has an option other than \"cache\"."))?;
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
        let known = object
            .keys()
            .all(|key| Self::CALL_KEYS.contains(&key.as_str()));
        known
            .then_some(())
            .ok_or_else(|| self.manual(line, "@WebCall has a key the fix does not know."))?;
        let adapter = object.get("client").and_then(Value::as_str);
        let path = object.get("path").and_then(Value::as_str);
        let verb = object
            .get("method")
            .and_then(Value::as_str)
            .and_then(|method| Self::VERBS.iter().find(|(verb, _)| *verb == method));
        let parts = adapter.zip(path).zip(verb);
        let ((adapter, path), (_, request)) = parts.ok_or_else(|| {
            self.manual(
                line,
                "@WebCall needs a client, a path and a GET, POST, PUT, PATCH or DELETE method.",
            )
        })?;
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
        let wanted = self.wanted_markers(object, line)?;
        if wanted.is_empty() {
            return Ok(Vec::new());
        }
        self.placed_markers(wanted, declaration, line)
    }

    fn wanted_markers(
        &self,
        object: &Map<String, Value>,
        line: usize,
    ) -> Result<Vec<(u64, String)>, ManualStep> {
        let mut wanted = Vec::new();
        for (key, marker) in [("query", "RequestParam"), ("headers", "RequestHeader")] {
            wanted.extend(self.mapped_markers(object, key, marker, line)?);
        }
        let body = object.get("body");
        let body_marker = body.map(|body| self.body_marker(body, line));
        wanted.extend(body_marker.transpose()?);
        Ok(wanted)
    }

    fn mapped_markers(
        &self,
        object: &Map<String, Value>,
        key: &str,
        marker: &str,
        line: usize,
    ) -> Result<Vec<(u64, String)>, ManualStep> {
        let Some(map) = object.get(key) else {
            return Ok(Vec::new());
        };
        let map = map
            .as_object()
            .ok_or_else(|| self.manual(line, "The fix cannot read the @WebCall mappings."))?;
        map.iter()
            .map(|(name, index)| self.mapped_marker(name, index, marker, line))
            .collect()
    }

    fn mapped_marker(
        &self,
        name: &str,
        index: &Value,
        marker: &str,
        line: usize,
    ) -> Result<(u64, String), ManualStep> {
        let index = index
            .as_u64()
            .ok_or_else(|| self.manual(line, "The fix cannot read the @WebCall mappings."))?;
        let written = Marker::spelled(self.extension, marker, &Marker::quoted(name));
        Ok((index, written))
    }

    fn body_marker(&self, body: &Value, line: usize) -> Result<(u64, String), ManualStep> {
        let index = body
            .as_u64()
            .ok_or_else(|| self.manual(line, "The fix cannot read the @WebCall body index."))?;
        Ok((index, Marker::spelled(self.extension, "RequestBody", "")))
    }

    fn placed_markers(
        &self,
        wanted: Vec<(u64, String)>,
        declaration: usize,
        line: usize,
    ) -> Result<Vec<TextEdit>, ManualStep> {
        let starts = self.parameter_starts(declaration).ok_or_else(|| {
            self.manual(line, "The fix cannot read the parameters of this method.")
        })?;
        let mut edits = Vec::new();
        for (index, marker) in wanted {
            let at = usize::try_from(index)
                .ok()
                .and_then(|index| starts.get(index))
                .ok_or_else(|| {
                    self.manual(line, "@WebCall names a parameter the method does not have.")
                })?;
            edits.push(self.placed_marker(*at, &marker, line)?);
        }
        let python = self.extension == "py";
        let import = python
            .then(|| PythonParameterEdits::import(self.text.source))
            .flatten();
        edits.extend(import);
        Ok(edits)
    }

    fn placed_marker(&self, at: usize, marker: &str, line: usize) -> Result<TextEdit, ManualStep> {
        match self.extension {
            "py" => {
                let marker = marker.trim_start_matches('@');
                PythonParameterEdits::wrap(&self.text, at, at, marker).ok_or_else(|| {
                    self.manual(
                        line,
                        "The fix cannot move this marker into the parameter's type.",
                    )
                })
            }
            "go" => Ok(TextEdit::insert(at, &format!("\n{marker}\n"))),
            _ => Ok(TextEdit::insert(at, &format!("{marker} "))),
        }
    }

    fn parameter_starts(&self, declaration: usize) -> Option<Vec<usize>> {
        let open = self.parameter_open(declaration)?;
        let close = self.text.matching(open)?;
        let separators = self.separators(open, close);
        let piece_starts = std::iter::once(open + 1).chain(separators.iter().map(|at| at + 1));
        let piece_ends = separators.iter().copied().chain(std::iter::once(close));
        let starts = piece_starts
            .zip(piece_ends)
            .filter_map(|(start, end)| self.parameter_start(start, end))
            .collect();
        Some(starts)
    }

    fn parameter_open(&self, declaration: usize) -> Option<usize> {
        let search = self.signature_start(declaration)?;
        let open = self.next_open(search)?;
        self.after_go_receiver(open)
    }

    fn signature_start(&self, declaration: usize) -> Option<usize> {
        if self.extension != "rs" {
            return Some(declaration);
        }
        Some(declaration + self.text.source[declaration..].find("fn ")?)
    }

    fn after_go_receiver(&self, open: usize) -> Option<usize> {
        if self.extension != "go" {
            return Some(open);
        }
        self.next_open(self.text.matching(open)? + 1)
    }

    fn separators(&self, open: usize, close: usize) -> Vec<usize> {
        let mut depth = 0_i32;
        let mut found = Vec::new();
        for at in (open + 1..=close).filter(|at| self.text.is_code(*at)) {
            found.extend(self.separator_at(at, close, &mut depth));
        }
        found
    }

    fn separator_at(&self, at: usize, close: usize, depth: &mut i32) -> Option<usize> {
        match self.text.bytes[at] {
            b'(' | b'[' | b'{' | b'<' => *depth += 1,
            b')' | b']' | b'}' | b'>' if at != close => *depth -= 1,
            b',' if *depth == 0 => return Some(at),
            _ => {}
        }
        None
    }

    fn parameter_start(&self, start: usize, end: usize) -> Option<usize> {
        let first = self.text.next_code(start).filter(|first| *first < end)?;
        let written = self.text.source[first..end].trim();
        (!self.is_receiver(written)).then_some(first)
    }

    fn is_receiver(&self, written: &str) -> bool {
        match self.extension {
            "py" => written == "self",
            "rs" => Self::RUST_RECEIVERS.contains(&written),
            _ => false,
        }
    }

    fn next_open(&self, from: usize) -> Option<usize> {
        (from..self.text.len()).find(|at| self.is_code_byte(*at, b'('))
    }

    fn is_code_byte(&self, at: usize, byte: u8) -> bool {
        [self.text.is_code(at), self.text.bytes[at] == byte] == [true, true]
    }

    fn is_name_character(character: char) -> bool {
        [character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }

    fn declared_name(&self, declaration: usize) -> Option<String> {
        let line = self.text.line_text(self.text.line_start(declaration));
        let words: Vec<&str> = line
            .split(|character: char| !Self::is_name_character(character))
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
        if self.extension == "go" {
            return self.go_receiver(declaration);
        }
        self.enclosing_type(declaration)
    }

    fn go_receiver(&self, declaration: usize) -> Option<String> {
        let line = self.text.line_text(self.text.line_start(declaration));
        let receiver = line.trim_start().strip_prefix("func (")?;
        let receiver = receiver.split(')').next()?;
        let name = receiver.split_whitespace().last()?.trim_start_matches('*');
        Some(String::from(name))
    }

    fn enclosing_type(&self, declaration: usize) -> Option<String> {
        let line_start = self.text.line_start(declaration);
        let first = self.text.previous_line_start(line_start);
        let mut previous_lines =
            std::iter::successors(first, |at| self.text.previous_line_start(*at));
        previous_lines.find_map(|line_start| self.type_on_line(line_start))
    }

    fn type_on_line(&self, line_start: usize) -> Option<String> {
        let at = line_start + self.text.indentation(line_start);
        self.text.is_code(at).then_some(())?;
        if self.extension == "rs" {
            return Self::impl_name(self.text.line_text(line_start).trim_start());
        }
        self.declared_name(at)
    }

    fn impl_name(trimmed: &str) -> Option<String> {
        let rest = trimmed.strip_prefix("impl")?;
        let name = rest
            .trim_start_matches(|character: char| !character.is_ascii_alphabetic())
            .split(|character: char| !Self::is_name_character(character))
            .next()?;
        Some(String::from(name))
    }

    fn declaration_of(&self, name: &str) -> Option<usize> {
        self.text
            .line_starts(0)
            .into_iter()
            .find_map(|line_start| self.declaration_at(line_start, name))
    }

    fn declaration_at(&self, line_start: usize, name: &str) -> Option<usize> {
        let at = line_start + self.text.indentation(line_start);
        let starts_impl = self
            .text
            .line_text(line_start)
            .trim_start()
            .starts_with("impl");
        let rust_impl = [self.extension == "rs", starts_impl] == [true, true];
        ([self.text.is_code(at), rust_impl] == [true, false]).then_some(())?;
        (self.declared_name(at).as_deref() == Some(name)).then_some(at)
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
