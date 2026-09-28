use super::super::{is_ignored_directory, portable_path, sorted_entries};
use super::ManualStep;
use super::edits::TextEdit;
use super::markers::Marker;
use super::scan::SourceText;
use crate::Failure;
use crate::routing::RoutePath;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RouteFile {
    pub(super) from: String,
    pub(super) to: String,
    template: String,
    handler: Option<String>,
}

#[derive(Default)]
pub(super) struct RouteMigration {
    pub(super) files: Vec<RouteFile>,
    pub(super) deletes: Vec<String>,
    pub(super) manual_steps: Vec<ManualStep>,
}

pub(super) struct RouteEdits {
    pub(super) edits: Vec<TextEdit>,
    pub(super) notes: Vec<String>,
    pub(super) steps: Vec<ManualStep>,
}

impl RouteMigration {
    pub(super) const ROOT: &'static str = "server/routes";
    const TARGET: &'static str = "server/controllers";
    const CONTROLLERS: &'static str = "controllers";
    const LANGUAGES: [&'static str; 10] = [
        "js", "ts", "py", "php", "java", "kt", "cs", "rs", "dart", "go",
    ];

    pub(super) fn plan(root: &Path) -> Result<Self, Failure> {
        let mut migration = Self::default();
        let routes = root.join(Self::ROOT);
        let Ok(metadata) = fs::symlink_metadata(&routes) else {
            return Ok(migration);
        };
        if metadata.is_symlink() || !metadata.is_dir() {
            migration.link(Self::ROOT);
            return Ok(migration);
        }
        let mut destinations = BTreeSet::new();
        migration.walk(root, &routes, &[], &mut destinations)?;
        Ok(migration)
    }

    pub(super) fn file(&self, relative: &str) -> Option<&RouteFile> {
        self.files.iter().find(|file| file.from == relative)
    }

    pub(super) fn is_allowed_move(from: &str, to: &str) -> bool {
        Self::target(from).is_some_and(|target| target == to)
    }

    pub(super) fn is_deletable(relative: &str) -> bool {
        let normal = Path::new(relative)
            .components()
            .all(|part| matches!(part, Component::Normal(_)));
        let Some(rest) = relative.strip_prefix(Self::ROOT) else {
            return false;
        };
        let name = rest.rsplit('/').next().unwrap_or("");
        let is_route_file = name
            .strip_prefix("yon.")
            .is_some_and(|extension| Self::LANGUAGES.contains(&extension));
        normal && rest.starts_with('/') && is_route_file
    }

    pub(super) fn is_blank(relative: &str, source: &str) -> bool {
        let extension = relative.rsplit('.').next().unwrap_or("");
        SourceText::read(extension, source).is_some_and(|text| text.next_code(0).is_none())
    }

    pub(super) fn remove_empty_folders(root: &Path) {
        let routes = root.join(Self::ROOT);
        let mut folders = Vec::new();
        let mut pending = vec![routes];
        while let Some(folder) = pending.pop() {
            let Ok(metadata) = fs::symlink_metadata(&folder) else {
                continue;
            };
            if metadata.is_symlink() || !metadata.is_dir() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(&folder) {
                for entry in entries.flatten() {
                    pending.push(entry.path());
                }
            }
            folders.push(folder);
        }
        for folder in folders.iter().rev() {
            let _ = fs::remove_dir(folder);
        }
    }

    fn target(from: &str) -> Option<String> {
        let rest = from.strip_prefix(Self::ROOT)?.strip_prefix('/')?;
        let mut segments: Vec<&str> = rest.split('/').collect();
        let name = segments.pop()?;
        let normal = Path::new(from)
            .components()
            .all(|part| matches!(part, Component::Normal(_)));
        if !normal {
            return None;
        }
        if segments.last() == Some(&Self::CONTROLLERS) {
            segments.pop();
            let extension = name.rsplit_once('.')?.1;
            if !Self::LANGUAGES.contains(&extension) {
                return None;
            }
            Self::template(&segments)?;
            return Some(format!("{}/{name}", Self::TARGET));
        }
        let extension = name.strip_prefix("yon.")?;
        if !Self::LANGUAGES.contains(&extension) {
            return None;
        }
        Self::template(&segments)?;
        Some(format!(
            "{}/{}Controller.{extension}",
            Self::TARGET,
            Self::class_stem(&segments)
        ))
    }

    fn walk(
        &mut self,
        root: &Path,
        folder: &Path,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) -> Result<(), Failure> {
        for entry in sorted_entries(folder)? {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let relative = portable_path(path.strip_prefix(root).unwrap_or(&path));
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.is_symlink() {
                self.link(&relative);
            } else if metadata.is_dir() {
                if is_ignored_directory(&name) {
                    continue;
                }
                if name == Self::CONTROLLERS {
                    self.controllers(root, &path, segments, destinations)?;
                    continue;
                }
                let mut inner = segments.to_vec();
                inner.push(name);
                self.walk(root, &path, &inner, destinations)?;
            } else {
                self.route_file(root, &relative, &name, segments, destinations);
            }
        }
        Ok(())
    }

    fn controllers(
        &mut self,
        root: &Path,
        folder: &Path,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) -> Result<(), Failure> {
        for entry in sorted_entries(folder)? {
            let path = entry.path();
            let relative = portable_path(path.strip_prefix(root).unwrap_or(&path));
            let is_file = fs::symlink_metadata(&path)
                .is_ok_and(|metadata| metadata.is_file() && !metadata.is_symlink());
            let Some(to) = Self::target(&relative).filter(|_| is_file) else {
                self.unread(&relative);
                continue;
            };
            let template = Self::template(segments).unwrap_or_default();
            self.add(root, &relative, to, template, None, destinations);
        }
        Ok(())
    }

    fn route_file(
        &mut self,
        root: &Path,
        relative: &str,
        name: &str,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) {
        if name == "OPTIONS.schema.json" {
            self.manual_steps.push(ManualStep::new(
                relative,
                String::from("Tachyon no longer reads OPTIONS.schema.json. The route schema now comes from the types in the controller method (ADR 0048)."),
                String::from("Give the method parameters @FromBody and model types with the same rules. Then delete this file."),
            ));
            return;
        }
        let Some(to) = Self::target(relative) else {
            self.unread(relative);
            return;
        };
        let path = root.join(relative);
        if !super::MigrationFix::is_small_file(&path).unwrap_or(false) {
            self.unread(relative);
            return;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            self.unread(relative);
            return;
        };
        if Self::is_blank(relative, &source) {
            self.deletes.push(String::from(relative));
            return;
        }
        let template = Self::template(segments).unwrap_or_default();
        let stem = Self::class_stem(segments);
        let class = format!("{stem}Controller");
        self.add(root, relative, to, template, Some(class), destinations);
    }

    fn add(
        &mut self,
        root: &Path,
        relative: &str,
        to: String,
        template: String,
        handler: Option<String>,
        destinations: &mut BTreeSet<String>,
    ) {
        let destination_key = to.to_lowercase();
        if destinations.contains(&destination_key) || root.join(&to).exists() {
            self.manual_steps.push(ManualStep::new(
                relative,
                format!("'{to}' already exists, so this route cannot move there (ADR 0048)."),
                format!("Move '{relative}' into server/controllers/ by hand under another name, or merge the two controllers."),
            ));
            return;
        }
        destinations.insert(destination_key);
        self.files.push(RouteFile {
            from: String::from(relative),
            to,
            template,
            handler,
        });
    }

    fn link(&mut self, relative: &str) {
        self.manual_steps.push(ManualStep::new(
            relative,
            String::from("This link is in server/routes, which Tachyon no longer reads. The fix does not follow links (ADR 0048)."),
            String::from("Replace the link with a controller in server/controllers/."),
        ));
    }

    fn unread(&mut self, relative: &str) {
        self.manual_steps.push(ManualStep::new(
            relative,
            String::from("This file is in server/routes, which Tachyon no longer reads (ADR 0048)."),
            String::from("Write its routes as a controller in server/controllers/, or move the file to the folder of its role."),
        ));
    }

    fn template(segments: &[impl AsRef<str>]) -> Option<String> {
        let mut parts = Vec::new();
        for segment in segments {
            let segment = segment.as_ref();
            match segment.strip_prefix('_') {
                Some(name) if RoutePath::valid_parameter_name(name) => {
                    parts.push(format!("{{{name}}}"));
                }
                None if RoutePath::valid_static_segment(segment) => {
                    parts.push(String::from(segment));
                }
                _ => return None,
            }
        }
        Some(format!("/{}", parts.join("/")))
    }

    fn class_stem(segments: &[impl AsRef<str>]) -> String {
        let mut stem = String::new();
        for segment in segments {
            for word in segment.as_ref().split(['-', '_']) {
                let mut characters = word.chars();
                if let Some(first) = characters.next() {
                    stem.push(first.to_ascii_uppercase());
                    stem.push_str(characters.as_str());
                }
            }
        }
        if stem.is_empty() {
            return String::from("Home");
        }
        stem
    }
}

impl RouteFile {
    const VERBS: [(&'static str, &'static str); 5] = [
        ("GET", "GetRoute"),
        ("POST", "PostRoute"),
        ("PUT", "PutRoute"),
        ("PATCH", "PatchRoute"),
        ("DELETE", "DeleteRoute"),
    ];
    const RETIRED_VERBS: [&'static str; 2] = ["HEAD", "OPTIONS"];
    const NOT_DECLARATIONS: [&'static str; 8] = [
        "return", "new", "await", "yield", "throw", "case", "not", "in",
    ];
    const RELATIVE_PATHS: [&'static str; 5] = ["../", "./", "__file__", "__dirname", "import.meta"];

    pub(super) fn edits(&self, source: &str) -> Result<RouteEdits, ManualStep> {
        let extension = self.from.rsplit('.').next().unwrap_or("");
        let Some(text) = SourceText::read(extension, source) else {
            return Err(self.manual(String::from("The fix cannot read this file.")));
        };
        let mut result = RouteEdits {
            edits: Vec::new(),
            notes: Vec::new(),
            steps: Vec::new(),
        };
        match &self.handler {
            Some(class) => self.handler_class(&text, extension, class, &mut result.edits)?,
            None => self.controller_marker(&text, extension, &mut result.edits)?,
        }
        let methods = Self::verb_methods(&text);
        if !methods
            .iter()
            .any(|(_, verb)| !Self::RETIRED_VERBS.contains(verb))
        {
            return Err(self.manual(String::from(
                "It has no GET, POST, PUT, PATCH or DELETE method.",
            )));
        }
        for (at, verb) in methods {
            if Self::RETIRED_VERBS.contains(&verb) {
                result.steps.push(ManualStep::new(
                    &format!("{}:{}", self.to, text.line_number(at)),
                    format!("A controller does not write a {verb} method. The server answers HEAD with the GET route, and OPTIONS with the route schema (ADR 0048)."),
                    format!("Delete the {verb} method."),
                ));
                continue;
            }
            self.route_method(&text, extension, at, verb, &mut result.edits)?;
        }
        if let Some(at) = Self::RELATIVE_PATHS
            .iter()
            .filter_map(|path| source.find(path))
            .min()
        {
            result.steps.push(ManualStep::new(
                &format!("{}:{}", self.to, text.line_number(at)),
                format!(
                    "This file moves from '{}', and it uses a relative path.",
                    self.from
                ),
                String::from("Check its relative paths and imports after the move."),
            ));
        }
        result.notes.push(format!(
            "Made the route {} a controller in {} (ADR 0048).",
            self.template, self.to
        ));
        Ok(result)
    }

    fn manual(&self, mut problem: String) -> ManualStep {
        problem.push_str(" So the fix cannot make it a controller (ADR 0048).");
        ManualStep::new(
            &self.from,
            problem,
            format!(
                "Move it to '{}' by hand. Mark the class @RequestRoute(\"{}\"), and mark each route method @GetRoute, @PostRoute and so on.",
                self.to, self.template
            ),
        )
    }

    fn controller_marker(
        &self,
        text: &SourceText<'_>,
        extension: &str,
        edits: &mut Vec<TextEdit>,
    ) -> Result<(), ManualStep> {
        let markers = Marker::lines(text, extension, &["controller"]);
        let [marker] = markers.as_slice() else {
            return Err(self.manual(String::from(
                "It does not have exactly one @Controller marker.",
            )));
        };
        let route = Marker::spelled(extension, "RequestRoute", &Marker::quoted(&self.template));
        let indentation = Marker::indentation(text, text.line_start(marker.start));
        let access = Marker::spelled(extension, "Public", "");
        edits.push(TextEdit::replace(
            marker.start,
            marker.end,
            &format!("{route}\n{indentation}{access}"),
        ));
        Ok(())
    }

    fn handler_class(
        &self,
        text: &SourceText<'_>,
        extension: &str,
        class: &str,
        edits: &mut Vec<TextEdit>,
    ) -> Result<(), ManualStep> {
        if extension == "rs" {
            return Err(self.manual(String::from(
                "A Rust controller needs a struct with a new function and &self methods.",
            )));
        }
        let uses = text.code_words("Handler");
        let Some(declaration) = uses.iter().copied().find(|at| {
            matches!(
                text.previous_word(*at),
                "class" | "object" | "type" | "struct"
            )
        }) else {
            return Err(self.manual(String::from("It has no class named Handler.")));
        };
        if matches!(extension, "js" | "ts") && !text.code_words("this").is_empty() {
            return Err(self.manual(String::from(
                "Its static methods use 'this', and route methods are not static.",
            )));
        }
        let keyword_end = text.previous_code(declaration).unwrap_or(0) + 1;
        let keyword_start = text.word_start(keyword_end);
        if &text.source[keyword_start..keyword_end] == "object" {
            let refers = uses
                .iter()
                .any(|at| text.bytes.get(at + "Handler".len()) == Some(&b'.'));
            if refers {
                return Err(self.manual(String::from(
                    "Other code uses 'Handler.' to reach members of the Kotlin object.",
                )));
            }
            edits.push(TextEdit::replace(keyword_start, keyword_end, "class"));
        }
        if text.previous_word(keyword_start) == "static" {
            let static_end = text.previous_code(keyword_start).unwrap_or(0) + 1;
            edits.push(TextEdit::delete(text.word_start(static_end), keyword_start));
        }
        for at in &uses {
            edits.push(TextEdit::replace(*at, at + "Handler".len(), class));
        }
        let line_start = text.line_start(declaration);
        let indentation = Marker::indentation(text, line_start);
        let route = Marker::spelled(extension, "RequestRoute", &Marker::quoted(&self.template));
        let access = Marker::spelled(extension, "Public", "");
        edits.push(TextEdit::insert(
            line_start,
            &format!("{indentation}{route}\n{indentation}{access}\n"),
        ));
        Ok(())
    }

    fn verb_methods(text: &SourceText<'_>) -> Vec<(usize, &'static str)> {
        let mut methods = Vec::new();
        let verbs = Self::VERBS.iter().map(|(verb, _)| *verb);
        for verb in verbs.chain(Self::RETIRED_VERBS) {
            for at in text.code_words(verb) {
                if Self::is_declaration(text, at, verb.len()) {
                    methods.push((at, verb));
                }
            }
        }
        methods.sort_unstable();
        methods
    }

    fn is_declaration(text: &SourceText<'_>, at: usize, length: usize) -> bool {
        let opens = text
            .next_code(at + length)
            .is_some_and(|open| text.bytes[open] == b'(');
        let line_start = text.line_start(at);
        let before = &text.source[line_start..at];
        let after_member = text.previous_code(at).is_some_and(|previous| {
            matches!(
                text.bytes[previous],
                b'.' | b':' | b'>' | b'=' | b'(' | b','
            )
        });
        let go_receiver = before.trim_start().starts_with("func (");
        let is_call = before.contains('=')
            || (before.contains('(') && !go_receiver)
            || Self::NOT_DECLARATIONS.contains(&text.previous_word(at));
        opens && !after_member && !is_call
    }

    fn route_method(
        &self,
        text: &SourceText<'_>,
        extension: &str,
        at: usize,
        verb: &str,
        edits: &mut Vec<TextEdit>,
    ) -> Result<(), ManualStep> {
        let line_start = text.line_start(at);
        let before = &text.source[line_start..at];
        if before.contains(['{', '}', ';']) {
            return Err(self.manual(format!("The {verb} method does not start its own line.")));
        }
        let lower = verb.to_ascii_lowercase();
        let name = if matches!(extension, "cs" | "go") {
            format!("{}{}", &verb[..1], &lower[1..])
        } else {
            lower
        };
        edits.push(TextEdit::replace(at, at + verb.len(), &name));
        if extension == "php" {
            Self::drop_unused_php_request(text, at + verb.len(), edits);
        }
        if self.handler.is_some() {
            Self::drop_static(text, extension, line_start, at, edits);
            if extension == "py" {
                let open = text.next_code(at + verb.len()).unwrap_or(at);
                let empty = text
                    .next_code(open + 1)
                    .is_some_and(|next| text.bytes[next] == b')');
                let receiver = if empty { "self" } else { "self, " };
                edits.push(TextEdit::insert(open + 1, receiver));
            }
        }
        let marker_name = Self::VERBS
            .iter()
            .find(|(old, _)| *old == verb)
            .map_or("GetRoute", |(_, marker)| marker);
        let indentation = Marker::indentation(text, line_start);
        let marker = Marker::spelled(extension, marker_name, "");
        edits.push(TextEdit::insert(
            line_start,
            &format!("{indentation}{marker}\n"),
        ));
        Ok(())
    }

    fn drop_unused_php_request(
        text: &SourceText<'_>,
        after_name: usize,
        edits: &mut Vec<TextEdit>,
    ) {
        let Some(open) = text.next_code(after_name) else {
            return;
        };
        if text.bytes[open] != b'(' {
            return;
        }
        let Some(close) = text.matching(open) else {
            return;
        };
        let parameter = text.source[open + 1..close].trim();
        let is_one_variable = parameter.starts_with('$')
            && parameter[1..]
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
        if is_one_variable && text.code_words(parameter).len() == 1 {
            edits.push(TextEdit::delete(open + 1, close));
        }
    }

    fn drop_static(
        text: &SourceText<'_>,
        extension: &str,
        line_start: usize,
        at: usize,
        edits: &mut Vec<TextEdit>,
    ) {
        let (word, marked) = match extension {
            "py" => ("staticmethod", true),
            "kt" => ("JvmStatic", true),
            "go" => return,
            _ => ("static", false),
        };
        let mut block_start = line_start;
        while let Some(previous) = text.previous_line_start(block_start) {
            if Marker::at_line(text, extension, previous).is_none() {
                break;
            }
            block_start = previous;
        }
        for found in text.code_words(word) {
            if found < block_start || found >= at {
                continue;
            }
            let start = if marked { found - 1 } else { found };
            let own_line = text.line_start(found);
            if own_line != line_start
                && text.line_text(own_line).trim() == &text.source[start..found + word.len()]
            {
                edits.push(TextEdit::delete(own_line, text.line_end(own_line)));
                continue;
            }
            let mut end = found + word.len();
            while text
                .bytes
                .get(end)
                .is_some_and(|byte| *byte == b' ' || *byte == b'\t')
            {
                end += 1;
            }
            edits.push(TextEdit::delete(start, end));
        }
    }
}
