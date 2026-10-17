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
        let configurations = migration.read_configurations(root);
        let manifest_text = migration.read_file(root, Self::MANIFEST);
        let manifest = migration.parse_manifest(manifest_text.as_deref());
        let has_old_member = manifest
            .as_ref()
            .is_some_and(|value| value.get("tachyon").is_some());
        let found = FoundFiles {
            legacy,
            configurations,
            manifest_text,
            has_old_member,
        };
        migration.finish(found, manifest.as_ref())
    }

    fn read_configurations(&mut self, root: &Path) -> Vec<(&'static str, String)> {
        let mut configurations = Vec::new();
        for name in Self::CONFIGURATION_FILES {
            let text = self.read_file(root, name);
            configurations.extend(text.map(|text| (name, text)));
        }
        configurations
    }

    fn finish(mut self, found: FoundFiles, manifest: Option<&Value>) -> Self {
        let nothing_to_move = [
            found.legacy.is_none(),
            found.configurations.is_empty(),
            found.has_old_member,
        ] == [true, true, false];
        let planned = manifest.map(PlannedManifest::read);
        match (nothing_to_move, planned) {
            (true, _) => Self::default(),
            (false, None) => self,
            (false, Some(None)) => {
                self.manual_steps.push(ManualStep::new(
                    Self::MANIFEST,
                    String::from("The manifest or its tac member is not a JSON object."),
                    String::from("Make both JSON objects, then run 'ty migrate fix' again."),
                ));
                self
            }
            (false, Some(Some(planned))) => self.apply(planned, found),
        }
    }

    fn apply(mut self, mut planned: PlannedManifest, found: FoundFiles) -> Self {
        let old_member_step = found.has_old_member.then(|| {
            ManualStep::new(
                Self::MANIFEST,
                String::from("The manifest member tachyon moved to tac.application (ADR 0052)."),
                String::from("Move its id, version and window into tac.application, then delete the tachyon member."),
            )
        });
        planned.manual_steps.extend(old_member_step);
        self.legacy_identity = found
            .legacy
            .filter(|text| planned.take_legacy_identity(text));
        for (name, text) in &found.configurations {
            planned.take_configuration(name, text);
        }
        self.original = found.manifest_text.unwrap_or_default();
        self.edits = planned.edits(&self.original);
        self.notes = planned.notes;
        self.manual_steps.extend(planned.manual_steps);
        self
    }

    fn read_file(&mut self, root: &Path, relative: &str) -> Option<String> {
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).ok()?;
        let small = metadata.len() <= Self::MAX_BYTES;
        let readable = [metadata.is_file(), metadata.is_symlink(), small] == [true, false, true];
        let text = readable.then(|| fs::read_to_string(&path).ok()).flatten();
        let step = text.is_none().then(|| {
            ManualStep::new(
                relative,
                String::from("This file is not a readable UTF-8 file of at most 1 MiB, so the fix cannot move its values (ADR 0052)."),
                format!("Move its values into {} by hand.", Self::MANIFEST),
            )
        });
        self.manual_steps.extend(step);
        text
    }

    fn parse_manifest(&mut self, text: Option<&str>) -> Option<Value> {
        let written = text.filter(|text| !text.trim().is_empty());
        let Some(written) = written else {
            return Some(Value::Object(Map::new()));
        };
        let parsed: Option<Value> = serde_json::from_str(crate::without_bom(written)).ok();
        let step = parsed.is_none().then(|| {
            ManualStep::new(
                Self::MANIFEST,
                String::from("The manifest is not valid JSON, so the fix cannot add values to it."),
                String::from("Correct its JSON, then run 'ty migrate fix' again."),
            )
        });
        self.manual_steps.extend(step);
        parsed
    }
}

struct FoundFiles {
    legacy: Option<String>,
    configurations: Vec<(&'static str, String)>,
    manifest_text: Option<String>,
    has_old_member: bool,
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
    const COMPUTED_TARGETS: [(&'static str, &'static str); 5] = [
        (
            "application",
            "name and start_url, and tac.application for id and version",
        ),
        ("metadata", "tac.metadata.routes"),
        ("origin", "tac.origin"),
        ("scripts", "tac.scripts"),
        ("styles", "tac.styles"),
    ];

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

    fn offer_root(&mut self, source: &str, key: &str, value: &Value) -> bool {
        let existing = self.root.get(key).cloned();
        let fresh = existing.is_none();
        self.root.entry(key).or_insert_with(|| value.clone());
        self.root_additions
            .extend(fresh.then(|| (String::from(key), value.clone())));
        self.notes.extend(fresh.then(|| Self::note(source, key)));
        existing.is_none_or(|existing| self.agrees(source, key, &existing, value))
    }

    fn offer_tac(&mut self, source: &str, key: &str, value: &Value) -> bool {
        let member = format!("tac.{key}");
        let existing = self.tac.get(key).cloned();
        let fresh = existing.is_none();
        self.tac.entry(key).or_insert_with(|| value.clone());
        self.tac_additions
            .extend(fresh.then(|| (String::from(key), value.clone())));
        self.notes
            .extend(fresh.then(|| Self::note(source, &member)));
        existing.is_none_or(|existing| self.agrees(source, &member, &existing, value))
    }

    fn agrees(&mut self, source: &str, member: &str, existing: &Value, value: &Value) -> bool {
        let same = existing == value;
        let step = (!same).then(|| {
            ManualStep::new(
                source,
                format!("{source} sets {member} to {value}, but client/manifest.json already has {existing}."),
                format!("Keep one value as {member} in client/manifest.json, then remove the other from {source}."),
            )
        });
        self.manual_steps.extend(step);
        same
    }

    fn note(source: &str, member: &str) -> String {
        format!("Adds {member} from {source} (ADR 0052).")
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
        let complete = [moved, has_only_application] == [true, true];
        let step = (!complete).then(|| {
            ManualStep::new(
                source,
                String::from(
                    "tachyon.json is no longer read, and not every value in it could be moved.",
                ),
                String::from("Finish the steps above, then delete tachyon.json."),
            )
        });
        self.manual_steps.extend(step);
        complete
    }

    fn take_application(&mut self, source: &str, application: &Map<String, Value>) -> bool {
        let mut identity = Map::new();
        let mut moved = true;
        for (key, value) in application {
            moved &= self.take_application_value(source, key, value, &mut identity);
        }
        let offered = (!identity.is_empty())
            .then(|| self.offer_tac(source, "application", &Value::Object(identity)));
        [moved, offered != Some(false)] == [true, true]
    }

    fn take_application_value(
        &mut self,
        source: &str,
        key: &str,
        value: &Value,
        identity: &mut Map<String, Value>,
    ) -> bool {
        match key {
            "name" => self.offer_root(source, "name", value),
            "entry_route" | "entryRoute" => self.offer_root(source, "start_url", value),
            "id" | "version" => {
                identity.insert(String::from(key), value.clone());
                true
            }
            _ => {
                self.manual_steps.push(ManualStep::new(
                    source,
                    format!("The application value '{key}' has no place in the manifest."),
                    String::from("Remove it, or move it by hand if it still matters."),
                ));
                false
            }
        }
    }

    fn take_configuration(&mut self, source: &str, text: &str) {
        for field in ManifestMigration::CONFIGURATION_FIELDS {
            self.take_field(source, text, field);
        }
        let post_bundle = StaticMember::find(text, "postBundle") != StaticMember::Missing;
        self.manual_steps.extend(post_bundle.then(|| {
            ManualStep::new(
                source,
                String::from("postBundle is removed; with tac.origin set, the build writes sitemap.xml and robots.txt itself (ADR 0052)."),
                String::from("Set tac.origin, and list the values of each page route with a path variable in tac.sitemap, such as \"/docs/_topic\": { \"data\": \"/shared/data/docs.json\", \"values\": \"/order\" }. Generate any other file before 'ty bundle'."),
            )
        }));
        self.manual_steps.push(ManualStep::new(
            source,
            format!("{source} is no longer read; client/manifest.json is the Tac configuration (ADR 0052)."),
            format!("Finish the steps for {source} above, then delete it."),
        ));
    }

    fn take_field(&mut self, source: &str, text: &str, field: &str) {
        match (StaticMember::find(text, field), field) {
            (StaticMember::Missing, _) => {}
            (StaticMember::Value(Value::Object(application)), "application") => {
                self.take_application(source, &application);
            }
            (StaticMember::Value(value), "metadata") => {
                let mut metadata = Map::new();
                metadata.insert(String::from("routes"), value);
                self.offer_tac(source, "metadata", &Value::Object(metadata));
            }
            (StaticMember::Value(value), "origin") if value.is_string() => {
                self.offer_tac(source, field, &value);
            }
            (StaticMember::Computed | StaticMember::Value(_), "application" | "origin")
            | (StaticMember::Computed, _) => {
                self.manual_steps.push(Self::computed_step(source, field));
            }
            (StaticMember::Value(value), _) => {
                self.offer_tac(source, field, &value);
            }
        }
    }

    fn computed_step(source: &str, field: &str) -> ManualStep {
        let known = Self::COMPUTED_TARGETS
            .iter()
            .find(|(name, _)| *name == field);
        let target = known.map_or("tac.cache", |(_, target)| *target);
        ManualStep::new(
            source,
            format!("static {field} is computed, so the fix cannot copy it as data."),
            format!("Write its final value into client/manifest.json as {target}."),
        )
    }

    fn edits(&self, original: &str) -> Vec<TextEdit> {
        let additions = [
            self.root_additions.is_empty(),
            self.tac_additions.is_empty(),
        ];
        let nothing_added = additions == [true, true];
        let empty_file = original.trim().is_empty();
        match (nothing_added, empty_file) {
            (true, _) => Vec::new(),
            (false, true) => vec![self.new_document(original)],
            (false, false) => self.existing_document_edits(original),
        }
    }

    fn rendered(members: &[(String, Value)], indent: usize) -> Vec<(String, String)> {
        members
            .iter()
            .map(|(key, value)| (key.clone(), JsonText::value(value, indent)))
            .collect()
    }

    fn new_document(&self, original: &str) -> TextEdit {
        let mut root_members = Self::rendered(&self.root_additions, 2);
        root_members.push((
            String::from("tac"),
            JsonText::object(&self.tac_additions, 2),
        ));
        let document = JsonText::rendered_object(&root_members, 0);
        TextEdit::replace(0, original.len(), &format!("{document}\n"))
    }

    fn existing_document_edits(&self, original: &str) -> Vec<TextEdit> {
        let has_tac_additions = !self.tac_additions.is_empty();
        let tac_object = JsonText::member_object(original, "tac").filter(|_| has_tac_additions);
        let tac_edit = tac_object.map(|(indent, close)| {
            let members = Self::rendered(&self.tac_additions, indent + 2);
            JsonText::insert_members(original, close, &members, indent)
        });
        let new_tac = [has_tac_additions, tac_object.is_none()] == [true, true];
        let mut root_members = Self::rendered(&self.root_additions, 2);
        root_members.extend(new_tac.then(|| {
            (
                String::from("tac"),
                JsonText::object(&self.tac_additions, 2),
            )
        }));
        let root_edit = (!root_members.is_empty()).then(|| {
            let close = original.trim_end().len() - 1;
            JsonText::insert_members(original, close, &root_members, 0)
        });
        tac_edit.into_iter().chain(root_edit).collect()
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
        let found = text
            .match_indices("static")
            .find_map(|(index, _)| Self::member_at(text, index, name));
        found.unwrap_or(Self::Missing)
    }

    fn member_at(text: &str, index: usize, name: &str) -> Option<Self> {
        let before = text[..index].chars().next_back();
        let inside_word = before.is_some_and(LiteralReader::is_word_character);
        (!inside_word).then_some(())?;
        let mut reader = LiteralReader::new(text, index + "static".len());
        reader
            .peek()
            .is_some_and(char::is_whitespace)
            .then_some(())?;
        reader.skip_space();
        reader.skip_async();
        reader.skip_space();
        (reader.word().as_deref() == Some(name)).then_some(())?;
        reader.skip_space();
        Self::declared(text, &reader)
    }

    fn declared(text: &str, reader: &LiteralReader<'_>) -> Option<Self> {
        match reader.peek() {
            Some('(') => Some(Self::Computed),
            Some('=') => Some(Self::read_value(text, reader.position + 1)),
            _ => None,
        }
    }

    fn read_value(text: &str, position: usize) -> Self {
        LiteralReader::read(text, position).map_or(Self::Computed, Self::Value)
    }
}

struct LiteralReader<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> LiteralReader<'a> {
    const MAX_DEPTH: usize = 32;
    const ASYNC: &'static str = "async";
    const STATEMENT_ENDS: [char; 2] = [';', '}'];
    const NUMBER_MARKS: [char; 5] = ['.', 'e', 'E', '+', '-'];

    fn new(text: &'a str, position: usize) -> Self {
        Self { text, position }
    }

    fn read(text: &'a str, position: usize) -> Option<Value> {
        let mut reader = Self::new(text, position);
        let value = reader.value(0)?;
        let crossed_line = reader.skip_space();
        let at_end = reader
            .peek()
            .is_none_or(|character| Self::STATEMENT_ENDS.contains(&character));
        [at_end, crossed_line].contains(&true).then_some(value)
    }

    fn is_word_character(character: char) -> bool {
        [
            character.is_ascii_alphanumeric(),
            character == '_',
            character == '$',
        ]
        .contains(&true)
    }

    fn is_number_character(character: char) -> bool {
        [
            character.is_ascii_digit(),
            Self::NUMBER_MARKS.contains(&character),
        ]
        .contains(&true)
    }

    fn skip_async(&mut self) {
        let rest = &self.text[self.position..];
        let after = rest.strip_prefix(Self::ASYNC);
        let whole_word = after.is_some_and(|after| !after.starts_with(Self::is_word_character));
        self.position += usize::from(whole_word) * Self::ASYNC.len();
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
        let starts_with_digit = word.starts_with(|character: char| character.is_ascii_digit());
        ([word.is_empty(), starts_with_digit] == [false, false]).then(|| String::from(word))
    }

    fn value(&mut self, depth: usize) -> Option<Value> {
        (depth <= Self::MAX_DEPTH).then_some(())?;
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
        while self.peek().is_some_and(Self::is_number_character) {
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
        let lines: Vec<String> = members
            .iter()
            .map(|(key, value)| format!("\n{}{}: {value}", " ".repeat(indent + 2), Self::key(key)))
            .collect();
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
        (text.as_bytes().get(open) == Some(&b'{')).then_some(())?;
        let close = Self::matching_close(text.as_bytes(), open)?;
        let line_start = text[..key_start].rfind('\n').map_or(0, |at| at + 1);
        let line = &text[line_start..key_start];
        Some((Self::key_indent(line), close))
    }

    fn key_indent(line_before_key: &str) -> usize {
        if line_before_key.trim().is_empty() {
            return line_before_key.len();
        }
        2
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
