use super::ManualStep;
use super::edits::TextEdit;
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

#[derive(Debug, Default)]
pub(super) struct ManifestMigration {
    pub(super) original: String,
    pub(super) edits: Vec<TextEdit>,
    pub(super) notes: Vec<String>,
    pub(super) legacy_identity: Option<String>,
    pub(super) manual_steps: Vec<ManualStep>,
}

impl ManifestMigration {
    pub(super) const MANIFEST: &str = "client/manifest.json";
    pub(super) const LEGACY_IDENTITY: &str = "tachyon.json";
    const CONFIGURATION_FILES: [&str; 3] = ["tac.config.js", "tac.config.mjs", "tac.config.ts"];
    const CONFIGURATION_FIELDS: [&str; 6] = [
        "origin",
        "application",
        "scripts",
        "styles",
        "cache",
        "metadata",
    ];
    const MAX_BYTES: u64 = 1_048_576;

    pub(super) fn plan(root: &Path) -> Self {
        let mut migration = Self::default();
        let legacy = migration.read_file(root, Self::LEGACY_IDENTITY);
        let mut configurations = Vec::new();
        for name in Self::CONFIGURATION_FILES {
            if let Some(text) = migration.read_file(root, name) {
                configurations.push((name, text));
            }
        }
        let manifest_text = migration.read_file(root, Self::MANIFEST);
        let manifest = migration.parse_manifest(manifest_text.as_deref());
        let has_old_member = manifest
            .as_ref()
            .is_some_and(|value| value.get("tachyon").is_some());
        if legacy.is_none() && configurations.is_empty() && !has_old_member {
            return Self::default();
        }
        let Some(manifest) = manifest else {
            return migration;
        };
        let Some(mut planned) = PlannedManifest::read(&manifest) else {
            migration.manual_steps.push(ManualStep::new(
                Self::MANIFEST,
                String::from("The manifest or its tac member is not a JSON object."),
                String::from("Make both JSON objects, then run 'ty migrate fix' again."),
            ));
            return migration;
        };
        if has_old_member {
            planned.manual_steps.push(ManualStep::new(
                Self::MANIFEST,
                String::from("The manifest member tachyon moved to tac.application (ADR 0052)."),
                String::from("Move its id, version and window into tac.application, then delete the tachyon member."),
            ));
        }
        if let Some(text) = legacy
            && planned.take_legacy_identity(&text)
        {
            migration.legacy_identity = Some(text);
        }
        for (name, text) in &configurations {
            planned.take_configuration(name, text);
        }
        migration.original = manifest_text.unwrap_or_default();
        migration.edits = planned.edits(&migration.original);
        migration.notes = planned.notes;
        migration.manual_steps.extend(planned.manual_steps);
        migration
    }

    fn read_file(&mut self, root: &Path, relative: &str) -> Option<String> {
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).ok()?;
        let readable =
            metadata.is_file() && !metadata.is_symlink() && metadata.len() <= Self::MAX_BYTES;
        let text = if readable {
            fs::read_to_string(&path).ok()
        } else {
            None
        };
        if text.is_none() {
            self.manual_steps.push(ManualStep::new(
                relative,
                String::from("This file is not a readable UTF-8 file of at most 1 MiB, so the fix cannot move its values (ADR 0052)."),
                format!("Move its values into {} by hand.", Self::MANIFEST),
            ));
        }
        text
    }

    fn parse_manifest(&mut self, text: Option<&str>) -> Option<Value> {
        let Some(text) = text else {
            return Some(Value::Object(Map::new()));
        };
        if text.trim().is_empty() {
            return Some(Value::Object(Map::new()));
        }
        let parsed = serde_json::from_str(crate::without_bom(text));
        if parsed.is_err() {
            self.manual_steps.push(ManualStep::new(
                Self::MANIFEST,
                String::from("The manifest is not valid JSON, so the fix cannot add values to it."),
                String::from("Correct its JSON, then run 'ty migrate fix' again."),
            ));
        }
        parsed.ok()
    }
}

struct PlannedManifest {
    root: Map<String, Value>,
    tac: Map<String, Value>,
    root_additions: Vec<(String, Value)>,
    tac_additions: Vec<(String, Value)>,
    notes: Vec<String>,
    manual_steps: Vec<ManualStep>,
}

impl PlannedManifest {
    fn read(manifest: &Value) -> Option<Self> {
        let root = manifest.as_object()?.clone();
        let tac = match root.get("tac") {
            Some(value) => value.as_object()?.clone(),
            None => Map::new(),
        };
        Some(Self {
            root,
            tac,
            root_additions: Vec::new(),
            tac_additions: Vec::new(),
            notes: Vec::new(),
            manual_steps: Vec::new(),
        })
    }

    fn offer_root(&mut self, source: &str, key: &str, value: Value) -> bool {
        if let Some(existing) = self.root.get(key) {
            let existing = existing.clone();
            return self.agrees(source, key, &existing, &value);
        }
        self.root.insert(String::from(key), value.clone());
        self.root_additions.push((String::from(key), value));
        self.note(source, key);
        true
    }

    fn offer_tac(&mut self, source: &str, key: &str, value: Value) -> bool {
        let member = format!("tac.{key}");
        if let Some(existing) = self.tac.get(key) {
            let existing = existing.clone();
            return self.agrees(source, &member, &existing, &value);
        }
        self.tac.insert(String::from(key), value.clone());
        self.tac_additions.push((String::from(key), value));
        self.note(source, &member);
        true
    }

    fn agrees(&mut self, source: &str, member: &str, existing: &Value, value: &Value) -> bool {
        if existing == value {
            return true;
        }
        self.manual_steps.push(ManualStep::new(
            source,
            format!("{source} sets {member} to {value}, but client/manifest.json already has {existing}."),
            format!("Keep one value as {member} in client/manifest.json, then remove the other from {source}."),
        ));
        false
    }

    fn note(&mut self, source: &str, member: &str) {
        self.notes
            .push(format!("Adds {member} from {source} (ADR 0052)."));
    }

    fn take_legacy_identity(&mut self, text: &str) -> bool {
        let source = ManifestMigration::LEGACY_IDENTITY;
        let document: Option<Value> = serde_json::from_str(crate::without_bom(text)).ok();
        let application = document
            .as_ref()
            .and_then(|value| value.get("application"))
            .and_then(Value::as_object);
        let Some(application) = application else {
            self.manual_steps.push(ManualStep::new(
                source,
                String::from("tachyon.json has no application object, so the fix cannot read it."),
                String::from(
                    "Move its values into client/manifest.json by hand, then delete tachyon.json.",
                ),
            ));
            return false;
        };
        let has_only_application = document
            .as_ref()
            .and_then(Value::as_object)
            .is_some_and(|members| members.len() == 1);
        let moved = self.take_application(source, application);
        if moved && has_only_application {
            return true;
        }
        self.manual_steps.push(ManualStep::new(
            source,
            String::from(
                "tachyon.json is no longer read, and not every value in it could be moved.",
            ),
            String::from("Finish the steps above, then delete tachyon.json."),
        ));
        false
    }

    fn take_application(&mut self, source: &str, application: &Map<String, Value>) -> bool {
        let mut identity = Map::new();
        let mut moved = true;
        for (key, value) in application {
            match key.as_str() {
                "name" => moved &= self.offer_root(source, "name", value.clone()),
                "entry_route" | "entryRoute" => {
                    moved &= self.offer_root(source, "start_url", value.clone());
                }
                "id" | "version" => {
                    identity.insert(key.clone(), value.clone());
                }
                _ => {
                    moved = false;
                    self.manual_steps.push(ManualStep::new(
                        source,
                        format!("The application value '{key}' has no place in the manifest."),
                        String::from("Remove it, or move it by hand if it still matters."),
                    ));
                }
            }
        }
        if !identity.is_empty() {
            moved &= self.offer_tac(source, "application", Value::Object(identity));
        }
        moved
    }

    fn take_configuration(&mut self, source: &str, text: &str) {
        for field in ManifestMigration::CONFIGURATION_FIELDS {
            match StaticMember::find(text, field) {
                StaticMember::Missing => {}
                StaticMember::Value(value) => self.take_configuration_value(source, field, &value),
                StaticMember::Computed => {
                    self.manual_steps.push(Self::computed_step(source, field));
                }
            }
        }
        if StaticMember::find(text, "postBundle") != StaticMember::Missing {
            self.manual_steps.push(ManualStep::new(
                source,
                String::from("postBundle is removed; with tac.origin set, the build writes sitemap.xml and robots.txt itself (ADR 0052)."),
                String::from("Set tac.origin, and list the values of each page route with a path variable in tac.sitemap, such as \"/docs/_topic\": { \"data\": \"/shared/data/docs.json\", \"values\": \"/order\" }. Generate any other file before 'ty bundle'."),
            ));
        }
        self.manual_steps.push(ManualStep::new(
            source,
            format!("{source} is no longer read; client/manifest.json is the Tac configuration (ADR 0052)."),
            format!("Finish the steps for {source} above, then delete it."),
        ));
    }

    fn take_configuration_value(&mut self, source: &str, field: &str, value: &Value) {
        match field {
            "application" => {
                if let Some(application) = value.as_object() {
                    self.take_application(source, application);
                } else {
                    self.manual_steps.push(Self::computed_step(source, field));
                }
            }
            "metadata" => {
                let mut metadata = Map::new();
                metadata.insert(String::from("routes"), value.clone());
                self.offer_tac(source, "metadata", Value::Object(metadata));
            }
            "origin" if !value.is_string() => {
                self.manual_steps.push(Self::computed_step(source, field));
            }
            _ => {
                self.offer_tac(source, field, value.clone());
            }
        }
    }

    fn computed_step(source: &str, field: &str) -> ManualStep {
        let target = match field {
            "application" => "name and start_url, and tac.application for id and version",
            "metadata" => "tac.metadata.routes",
            "origin" => "tac.origin",
            "scripts" => "tac.scripts",
            "styles" => "tac.styles",
            _ => "tac.cache",
        };
        ManualStep::new(
            source,
            format!("static {field} is computed, so the fix cannot copy it as data."),
            format!("Write its final value into client/manifest.json as {target}."),
        )
    }

    fn edits(&self, original: &str) -> Vec<TextEdit> {
        if self.root_additions.is_empty() && self.tac_additions.is_empty() {
            return Vec::new();
        }
        let mut root_members = Vec::new();
        for (key, value) in &self.root_additions {
            root_members.push((key.clone(), JsonText::value(value, 2)));
        }
        if original.trim().is_empty() {
            root_members.push((
                String::from("tac"),
                JsonText::object(&self.tac_additions, 2),
            ));
            let document = JsonText::rendered_object(&root_members, 0);
            return vec![TextEdit::replace(
                0,
                original.len(),
                &format!("{document}\n"),
            )];
        }
        let mut edits = Vec::new();
        match JsonText::member_object(original, "tac") {
            Some((indent, close)) if !self.tac_additions.is_empty() => {
                let mut members = Vec::new();
                for (key, value) in &self.tac_additions {
                    members.push((key.clone(), JsonText::value(value, indent + 2)));
                }
                edits.push(JsonText::insert_members(original, close, &members, indent));
            }
            None if !self.tac_additions.is_empty() => {
                root_members.push((
                    String::from("tac"),
                    JsonText::object(&self.tac_additions, 2),
                ));
            }
            _ => {}
        }
        if !root_members.is_empty() {
            let close = original.trim_end().len() - 1;
            edits.push(JsonText::insert_members(original, close, &root_members, 0));
        }
        edits
    }
}

#[derive(Debug, PartialEq)]
enum StaticMember {
    Missing,
    Value(Value),
    Computed,
}

impl StaticMember {
    fn find(text: &str, name: &str) -> Self {
        for (index, _) in text.match_indices("static") {
            let before = text[..index].chars().next_back();
            if before.is_some_and(LiteralReader::is_word_character) {
                continue;
            }
            let mut reader = LiteralReader::new(text, index + "static".len());
            if !reader.peek().is_some_and(char::is_whitespace) {
                continue;
            }
            reader.skip_space();
            let mut word = reader.word();
            if word.as_deref() == Some("async") {
                reader.skip_space();
                word = reader.word();
            }
            if word.as_deref() != Some(name) {
                continue;
            }
            reader.skip_space();
            match reader.peek() {
                Some('(') => return Self::Computed,
                Some('=') => return Self::read_value(text, reader.position + 1),
                _ => {}
            }
        }
        Self::Missing
    }

    fn read_value(text: &str, position: usize) -> Self {
        match LiteralReader::read(text, position) {
            Some(value) => Self::Value(value),
            None => Self::Computed,
        }
    }
}

struct LiteralReader<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> LiteralReader<'a> {
    const MAX_DEPTH: usize = 32;

    fn new(text: &'a str, position: usize) -> Self {
        Self { text, position }
    }

    fn read(text: &'a str, position: usize) -> Option<Value> {
        let mut reader = Self::new(text, position);
        let value = reader.value(0)?;
        let crossed_line = reader.skip_space();
        let ends = match reader.peek() {
            None | Some(';' | '}') => true,
            Some(_) => crossed_line,
        };
        if ends { Some(value) } else { None }
    }

    fn is_word_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || character == '_' || character == '$'
    }

    fn peek(&self) -> Option<char> {
        self.text[self.position..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.position += character.len_utf8();
        Some(character)
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.position += expected.len_utf8();
            return true;
        }
        false
    }

    fn skip_space(&mut self) -> bool {
        let mut crossed_line = false;
        loop {
            let rest = &self.text[self.position..];
            if rest.starts_with("//") {
                let end = rest.find('\n').unwrap_or(rest.len());
                self.position += end;
                continue;
            }
            if rest.starts_with("/*") {
                let Some(end) = rest.find("*/") else {
                    self.position = self.text.len();
                    return crossed_line;
                };
                crossed_line |= rest[..end].contains('\n');
                self.position += end + 2;
                continue;
            }
            match self.peek() {
                Some(character) if character.is_whitespace() => {
                    crossed_line |= character == '\n';
                    self.position += character.len_utf8();
                }
                _ => return crossed_line,
            }
        }
    }

    fn word(&mut self) -> Option<String> {
        let start = self.position;
        while self.peek().is_some_and(Self::is_word_character) {
            self.position += 1;
        }
        let word = &self.text[start..self.position];
        if word.is_empty() || word.starts_with(|character: char| character.is_ascii_digit()) {
            return None;
        }
        Some(String::from(word))
    }

    fn value(&mut self, depth: usize) -> Option<Value> {
        if depth > Self::MAX_DEPTH {
            return None;
        }
        self.skip_space();
        match self.peek()? {
            '{' => self.object(depth),
            '[' => self.array(depth),
            '\'' | '"' => self.quoted().map(Value::String),
            '`' => self.template().map(Value::String),
            '-' | '0'..='9' => self.number(),
            _ => self.keyword(),
        }
    }

    fn keyword(&mut self) -> Option<Value> {
        match self.word()?.as_str() {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            "null" => Some(Value::Null),
            _ => None,
        }
    }

    fn object(&mut self, depth: usize) -> Option<Value> {
        self.position += 1;
        let mut members = Map::new();
        loop {
            self.skip_space();
            if self.eat('}') {
                return Some(Value::Object(members));
            }
            let key = match self.peek()? {
                '\'' | '"' => self.quoted()?,
                _ => self.word()?,
            };
            self.skip_space();
            if !self.eat(':') {
                return None;
            }
            let value = self.value(depth + 1)?;
            members.insert(key, value);
            self.skip_space();
            if self.eat(',') {
                continue;
            }
            if self.eat('}') {
                return Some(Value::Object(members));
            }
            return None;
        }
    }

    fn array(&mut self, depth: usize) -> Option<Value> {
        self.position += 1;
        let mut items = Vec::new();
        loop {
            self.skip_space();
            if self.eat(']') {
                return Some(Value::Array(items));
            }
            items.push(self.value(depth + 1)?);
            self.skip_space();
            if self.eat(',') {
                continue;
            }
            if self.eat(']') {
                return Some(Value::Array(items));
            }
            return None;
        }
    }

    fn quoted(&mut self) -> Option<String> {
        let quote = self.advance()?;
        let mut text = String::new();
        loop {
            let character = self.advance()?;
            if character == quote {
                return Some(text);
            }
            if character == '\n' {
                return None;
            }
            if character == '\\' {
                text.push(self.escaped()?);
            } else {
                text.push(character);
            }
        }
    }

    fn template(&mut self) -> Option<String> {
        self.position += 1;
        let mut text = String::new();
        loop {
            let character = self.advance()?;
            if character == '`' {
                return Some(text);
            }
            if character == '$' && self.peek() == Some('{') {
                return None;
            }
            if character == '\\' {
                text.push(self.escaped()?);
            } else {
                text.push(character);
            }
        }
    }

    fn escaped(&mut self) -> Option<char> {
        let character = self.advance()?;
        let plain = match character {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'v' => '\u{b}',
            '0' => '\0',
            'u' => return self.unicode(),
            '\\' | '\'' | '"' | '`' | '/' | '$' => character,
            _ => return None,
        };
        Some(plain)
    }

    fn unicode(&mut self) -> Option<char> {
        let digits = self.text.get(self.position..self.position + 4)?;
        let code = u32::from_str_radix(digits, 16).ok()?;
        self.position += 4;
        char::from_u32(code)
    }

    fn number(&mut self) -> Option<Value> {
        let start = self.position;
        while self.peek().is_some_and(|character| {
            character.is_ascii_digit() || matches!(character, '.' | 'e' | 'E' | '+' | '-')
        }) {
            self.position += 1;
        }
        let number: serde_json::Number =
            serde_json::from_str(&self.text[start..self.position]).ok()?;
        Some(Value::Number(number))
    }
}

struct JsonText;

impl JsonText {
    fn value(value: &Value, indent: usize) -> String {
        let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| String::from("null"));
        pretty.replace('\n', &format!("\n{}", " ".repeat(indent)))
    }

    fn object(members: &[(String, Value)], indent: usize) -> String {
        let mut rendered = Vec::new();
        for (key, value) in members {
            rendered.push((key.clone(), Self::value(value, indent + 2)));
        }
        Self::rendered_object(&rendered, indent)
    }

    fn rendered_object(members: &[(String, String)], indent: usize) -> String {
        let mut lines = Vec::new();
        for (key, value) in members {
            lines.push(format!(
                "{}{}: {value}",
                " ".repeat(indent + 2),
                Self::key(key)
            ));
        }
        format!("{{\n{}\n{}}}", lines.join(",\n"), " ".repeat(indent))
    }

    fn key(key: &str) -> String {
        serde_json::to_string(key).unwrap_or_else(|_| format!("\"{key}\""))
    }

    fn insert_members(
        original: &str,
        close: usize,
        members: &[(String, String)],
        indent: usize,
    ) -> TextEdit {
        let before = original[..close].trim_end();
        let last = before.len();
        let mut lines = Vec::new();
        for (key, value) in members {
            lines.push(format!(
                "\n{}{}: {value}",
                " ".repeat(indent + 2),
                Self::key(key)
            ));
        }
        if before.ends_with('{') {
            let replacement = format!("{}\n{}", lines.join(","), " ".repeat(indent));
            return TextEdit::replace(last, close, &replacement);
        }
        TextEdit::insert(last, &format!(",{}", lines.join(",")))
    }

    fn member_object(text: &str, name: &str) -> Option<(usize, usize)> {
        let bytes = text.as_bytes();
        let mut depth = 0_usize;
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'"' => {
                    let end = Self::string_end(bytes, index)?;
                    let after = text[end + 1..].trim_start();
                    let is_wanted_key =
                        depth == 1 && after.starts_with(':') && &text[index + 1..end] == name;
                    if is_wanted_key {
                        return Self::object_after_key(text, index, end);
                    }
                    index = end;
                }
                b'{' | b'[' => depth += 1,
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
            index += 1;
        }
        None
    }

    fn object_after_key(text: &str, key_start: usize, key_end: usize) -> Option<(usize, usize)> {
        let colon = key_end + 1 + text[key_end + 1..].find(':')?;
        let open = colon + 1 + (text[colon + 1..].len() - text[colon + 1..].trim_start().len());
        if text.as_bytes().get(open) != Some(&b'{') {
            return None;
        }
        let close = Self::matching_close(text.as_bytes(), open)?;
        let line_start = text[..key_start].rfind('\n').map_or(0, |at| at + 1);
        let line = &text[line_start..key_start];
        let indent = if line.trim().is_empty() {
            line.len()
        } else {
            2
        };
        Some((indent, close))
    }

    fn string_end(bytes: &[u8], start: usize) -> Option<usize> {
        let mut index = start + 1;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' => index += 2,
                b'"' => return Some(index),
                _ => index += 1,
            }
        }
        None
    }

    fn matching_close(bytes: &[u8], open: usize) -> Option<usize> {
        let mut depth = 0_usize;
        let mut index = open;
        while index < bytes.len() {
            match bytes[index] {
                b'"' => index = Self::string_end(bytes, index)?,
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
            index += 1;
        }
        None
    }
}
