use super::super::{is_ignored_directory, portable_path, sorted_entries};
use super::edits::TextEdit;
use super::markers::Marker;
use super::scan::SourceText;
use super::{ManualStep, MigrationFix};
use crate::Failure;
use crate::routing::RoutePath;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

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
    const SCHEMA: &'static str = "OPTIONS.schema.json";
    const LANGUAGES: [&'static str; 10] = [
        "js", "ts", "py", "php", "java", "kt", "cs", "rs", "dart", "go",
    ];

    pub(super) fn plan(root: &Path) -> Result<Self, Failure> {
        let mut migration = Self::default();
        let routes = root.join(Self::ROOT);
        let metadata = fs::symlink_metadata(&routes).ok();
        let real_folder =
            metadata.map(|metadata| [metadata.is_symlink(), metadata.is_dir()] == [false, true]);
        migration.read_routes(root, &routes, real_folder)?;
        Ok(migration)
    }

    fn read_routes(
        &mut self,
        root: &Path,
        routes: &Path,
        real_folder: Option<bool>,
    ) -> Result<(), Failure> {
        match real_folder {
            None => Ok(()),
            Some(false) => {
                self.link(Self::ROOT);
                Ok(())
            }
            Some(true) => {
                let mut destinations = BTreeSet::new();
                self.walk(root, routes, &[], &mut destinations)
            }
        }
    }

    pub(super) fn file(&self, relative: &str) -> Option<&RouteFile> {
        self.files.iter().find(|file| file.from == relative)
    }

    pub(super) fn is_allowed_move(from: &str, to: &str) -> bool {
        Self::target(from).is_some_and(|target| target == to)
    }

    pub(super) fn is_deletable(relative: &str) -> bool {
        let normal = MigrationFix::is_normal(relative);
        let rest = relative.strip_prefix(Self::ROOT);
        let in_routes = rest.is_some_and(|rest| rest.starts_with('/'));
        let name = rest.and_then(|rest| rest.rsplit('/').next()).unwrap_or("");
        let is_route_file = name
            .strip_prefix("yon.")
            .is_some_and(|extension| Self::LANGUAGES.contains(&extension));
        [normal, in_routes, is_route_file] == [true; 3]
    }

    pub(super) fn is_blank(relative: &str, source: &str) -> bool {
        let extension = relative.rsplit('.').next().unwrap_or("");
        SourceText::read(extension, source).is_some_and(|text| text.next_code(0).is_none())
    }

    pub(super) fn remove_empty_folders(root: &Path) {
        let folders = Self::route_folders(root.join(Self::ROOT));
        Self::remove_folders(&folders);
    }

    fn route_folders(routes: PathBuf) -> Vec<PathBuf> {
        let mut folders = Vec::new();
        let mut pending = vec![routes];
        while let Some(folder) = pending.pop() {
            let entries = Self::real_folder_entries(&folder);
            folders.extend(entries.is_some().then_some(folder));
            pending.extend(entries.into_iter().flatten());
        }
        folders
    }

    fn real_folder_entries(folder: &Path) -> Option<Vec<PathBuf>> {
        let real = fs::symlink_metadata(folder)
            .is_ok_and(|metadata| [metadata.is_symlink(), metadata.is_dir()] == [false, true]);
        real.then(|| {
            fs::read_dir(folder).map_or_else(
                |_| Vec::new(),
                |entries| entries.flatten().map(|entry| entry.path()).collect(),
            )
        })
    }

    fn remove_folders(folders: &[PathBuf]) {
        for folder in folders.iter().rev() {
            let _ = fs::remove_dir(folder);
        }
    }

    fn target(from: &str) -> Option<String> {
        let rest = from.strip_prefix(Self::ROOT)?.strip_prefix('/')?;
        let mut segments: Vec<&str> = rest.split('/').collect();
        let name = segments.pop()?;
        MigrationFix::is_normal(from).then_some(())?;
        if let Some((&Self::CONTROLLERS, parents)) = segments.split_last() {
            return Self::controller_target(parents, name);
        }
        Self::route_target(&segments, name)
    }

    fn controller_target(segments: &[&str], name: &str) -> Option<String> {
        let extension = name.rsplit_once('.')?.1;
        Self::LANGUAGES.contains(&extension).then_some(())?;
        Self::template(segments)?;
        Some(format!("{}/{name}", Self::TARGET))
    }

    fn route_target(segments: &[&str], name: &str) -> Option<String> {
        let extension = name.strip_prefix("yon.")?;
        Self::LANGUAGES.contains(&extension).then_some(())?;
        Self::template(segments)?;
        Some(format!(
            "{}/{}Controller.{extension}",
            Self::TARGET,
            Self::class_stem(segments)
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
            self.walk_entry(root, &entry, segments, destinations)?;
        }
        Ok(())
    }

    fn walk_entry(
        &mut self,
        root: &Path,
        entry: &fs::DirEntry,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) -> Result<(), Failure> {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let relative = portable_path(path.strip_prefix(root).unwrap_or(&path));
        let metadata = fs::symlink_metadata(&path).ok();
        let kind = metadata.map(|metadata| (metadata.is_symlink(), metadata.is_dir()));
        match kind {
            None => Ok(()),
            Some((true, _)) => {
                self.link(&relative);
                Ok(())
            }
            Some((false, true)) if is_ignored_directory(&name) => Ok(()),
            Some((false, true)) if name == Self::CONTROLLERS => {
                self.controllers(root, &path, segments, destinations)
            }
            Some((false, true)) => self.walk_deeper(root, &path, name, segments, destinations),
            Some((false, false)) if name == Self::SCHEMA => {
                self.schema_step(&relative);
                Ok(())
            }
            Some((false, false)) => {
                self.route_file(root, &relative, segments, destinations);
                Ok(())
            }
        }
    }

    fn walk_deeper(
        &mut self,
        root: &Path,
        folder: &Path,
        name: String,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) -> Result<(), Failure> {
        let mut inner = segments.to_vec();
        inner.push(name);
        self.walk(root, folder, &inner, destinations)
    }

    fn controllers(
        &mut self,
        root: &Path,
        folder: &Path,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) -> Result<(), Failure> {
        for entry in sorted_entries(folder)? {
            self.controller_entry(root, &entry, segments, destinations);
        }
        Ok(())
    }

    fn controller_entry(
        &mut self,
        root: &Path,
        entry: &fs::DirEntry,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) {
        let path = entry.path();
        let relative = portable_path(path.strip_prefix(root).unwrap_or(&path));
        let is_file = fs::symlink_metadata(&path)
            .is_ok_and(|metadata| [metadata.is_file(), metadata.is_symlink()] == [true, false]);
        let to = Self::target(&relative).filter(|_| is_file);
        self.add_controller(root, &relative, to, segments, destinations);
    }

    fn add_controller(
        &mut self,
        root: &Path,
        relative: &str,
        to: Option<String>,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) {
        let Some(to) = to else {
            self.unread(relative);
            return;
        };
        let template = Self::template(segments).unwrap_or_default();
        self.add(root, relative, to, template, None, destinations);
    }

    fn schema_step(&mut self, relative: &str) {
        self.manual_steps.push(ManualStep::new(
            relative,
            String::from("Tachyon no longer reads OPTIONS.schema.json. The route schema now comes from the types in the controller method (ADR 0048)."),
            String::from("Give the method parameters @FromBody and model types with the same rules. Then delete this file."),
        ));
    }

    fn route_file(
        &mut self,
        root: &Path,
        relative: &str,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) {
        let read = Self::read_route(root, relative);
        self.add_read_route(root, relative, read, segments, destinations);
    }

    fn read_route(root: &Path, relative: &str) -> Option<(String, String)> {
        let to = Self::target(relative)?;
        let path = root.join(relative);
        let small = MigrationFix::is_small_file(&path).unwrap_or(false);
        small.then_some(())?;
        let source = fs::read_to_string(&path).ok()?;
        Some((to, source))
    }

    fn add_read_route(
        &mut self,
        root: &Path,
        relative: &str,
        read: Option<(String, String)>,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) {
        match read {
            None => self.unread(relative),
            Some((_, source)) if Self::is_blank(relative, &source) => {
                self.deletes.push(String::from(relative));
            }
            Some((to, _)) => self.add_route(root, relative, to, segments, destinations),
        }
    }

    fn add_route(
        &mut self,
        root: &Path,
        relative: &str,
        to: String,
        segments: &[String],
        destinations: &mut BTreeSet<String>,
    ) {
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
        let claimed = destinations.contains(&destination_key);
        let taken = [claimed, root.join(&to).exists()].contains(&true);
        let step = taken.then(|| {
            ManualStep::new(
                relative,
                format!("'{to}' already exists, so this route cannot move there (ADR 0048)."),
                format!("Move '{relative}' into server/controllers/ by hand under another name, or merge the two controllers."),
            )
        });
        self.manual_steps.extend(step);
        let file = RouteFile {
            from: String::from(relative),
            to,
            template,
            handler,
        };
        destinations.extend((!taken).then_some(destination_key));
        self.files.extend((!taken).then_some(file));
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
        let parts: Option<Vec<String>> = segments
            .iter()
            .map(|segment| Self::template_part(segment.as_ref()))
            .collect();
        Some(format!("/{}", parts?.join("/")))
    }

    fn template_part(segment: &str) -> Option<String> {
        match segment.strip_prefix('_') {
            Some(name) if RoutePath::valid_parameter_name(name) => Some(format!("{{{name}}}")),
            None if RoutePath::valid_static_segment(segment) => Some(String::from(segment)),
            _ => None,
        }
    }

    fn class_stem(segments: &[impl AsRef<str>]) -> String {
        let stem: String = segments
            .iter()
            .flat_map(|segment| segment.as_ref().split(['-', '_']))
            .map(Self::capitalized)
            .collect();
        if stem.is_empty() {
            return String::from("Home");
        }
        stem
    }

    fn capitalized(word: &str) -> String {
        let mut characters = word.chars();
        let first = characters.next().map(|first| first.to_ascii_uppercase());
        let first: String = first.into_iter().collect();
        format!("{first}{}", characters.as_str())
    }
}

impl RouteFile {
    const HANDLER: &'static str = "Handler";
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
    const TYPE_WORDS: [&'static str; 4] = ["class", "object", "type", "struct"];
    const SCRIPTS: [&'static str; 2] = ["js", "ts"];
    const CAPITALIZED_METHODS: [&'static str; 2] = ["cs", "go"];
    const MEMBER_MARKS: &'static [u8] = b".:>=(,";
    const SPACES: &'static [u8] = b" \t";

    pub(super) fn edits(&self, source: &str) -> Result<RouteEdits, ManualStep> {
        let extension = self.from.rsplit('.').next().unwrap_or("");
        let text = SourceText::read(extension, source)
            .ok_or_else(|| self.manual(String::from("The fix cannot read this file.")))?;
        let mut result = RouteEdits {
            edits: Vec::new(),
            notes: Vec::new(),
            steps: Vec::new(),
        };
        self.class_edits(&text, extension, &mut result.edits)?;
        let methods = Self::verb_methods(&text);
        let has_route = methods
            .iter()
            .any(|(_, verb)| !Self::RETIRED_VERBS.contains(verb));
        self.refuse(
            !has_route,
            "It has no GET, POST, PUT, PATCH or DELETE method.",
        )?;
        self.add_methods(&text, extension, methods, &mut result)?;
        result.steps.extend(self.relative_path_step(&text, source));
        result.notes.push(format!(
            "Made the route {} a controller in {} (ADR 0048).",
            self.template, self.to
        ));
        Ok(result)
    }

    fn refuse(&self, found: bool, problem: &str) -> Result<(), ManualStep> {
        (!found)
            .then_some(())
            .ok_or_else(|| self.manual(String::from(problem)))
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

    fn class_edits(
        &self,
        text: &SourceText<'_>,
        extension: &str,
        edits: &mut Vec<TextEdit>,
    ) -> Result<(), ManualStep> {
        match &self.handler {
            Some(class) => self.handler_class(text, extension, class, edits),
            None => self.controller_marker(text, extension, edits),
        }
    }

    fn add_methods(
        &self,
        text: &SourceText<'_>,
        extension: &str,
        methods: Vec<(usize, &'static str)>,
        result: &mut RouteEdits,
    ) -> Result<(), ManualStep> {
        for (at, verb) in methods {
            self.method_edits(text, extension, at, verb, result)?;
        }
        Ok(())
    }

    fn method_edits(
        &self,
        text: &SourceText<'_>,
        extension: &str,
        at: usize,
        verb: &str,
        result: &mut RouteEdits,
    ) -> Result<(), ManualStep> {
        if Self::RETIRED_VERBS.contains(&verb) {
            result.steps.push(ManualStep::new(
                &format!("{}:{}", self.to, text.line_number(at)),
                format!("A controller does not write a {verb} method. The server answers HEAD with the GET route, and OPTIONS with the route schema (ADR 0048)."),
                format!("Delete the {verb} method."),
            ));
            return Ok(());
        }
        self.route_method(text, extension, at, verb, &mut result.edits)
    }

    fn relative_path_step(&self, text: &SourceText<'_>, source: &str) -> Option<ManualStep> {
        let at = Self::RELATIVE_PATHS
            .iter()
            .filter_map(|path| source.find(path))
            .min()?;
        Some(ManualStep::new(
            &format!("{}:{}", self.to, text.line_number(at)),
            format!(
                "This file moves from '{}', and it uses a relative path.",
                self.from
            ),
            String::from("Check its relative paths and imports after the move."),
        ))
    }

    fn controller_marker(
        &self,
        text: &SourceText<'_>,
        extension: &str,
        edits: &mut Vec<TextEdit>,
    ) -> Result<(), ManualStep> {
        let markers = Marker::lines(text, extension, &["controller"]);
        let single = markers.first().filter(|_| markers.len() == 1);
        let marker = single.ok_or_else(|| {
            self.manual(String::from(
                "It does not have exactly one @Controller marker.",
            ))
        })?;
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
        self.refuse(
            extension == "rs",
            "A Rust controller needs a struct with a new function and &self methods.",
        )?;
        let uses = text.code_words(Self::HANDLER);
        let declaration = uses
            .iter()
            .copied()
            .find(|at| Self::TYPE_WORDS.contains(&text.previous_word(*at)))
            .ok_or_else(|| self.manual(String::from("It has no class named Handler.")))?;
        let script = Self::SCRIPTS.contains(&extension);
        let uses_this = !text.code_words("this").is_empty();
        self.refuse(
            [script, uses_this] == [true, true],
            "Its static methods use 'this', and route methods are not static.",
        )?;
        let keyword_end = text.previous_code(declaration).unwrap_or(0) + 1;
        let keyword_start = text.word_start(keyword_end);
        self.kotlin_object_edit(text, &uses, keyword_start, keyword_end, edits)?;
        edits.extend(Self::static_class_edit(text, keyword_start));
        let renamed = uses
            .iter()
            .map(|at| TextEdit::replace(*at, at + Self::HANDLER.len(), class));
        edits.extend(renamed);
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

    fn kotlin_object_edit(
        &self,
        text: &SourceText<'_>,
        uses: &[usize],
        keyword_start: usize,
        keyword_end: usize,
        edits: &mut Vec<TextEdit>,
    ) -> Result<(), ManualStep> {
        if &text.source[keyword_start..keyword_end] != "object" {
            return Ok(());
        }
        let refers = uses
            .iter()
            .any(|at| text.bytes.get(at + Self::HANDLER.len()) == Some(&b'.'));
        self.refuse(
            refers,
            "Other code uses 'Handler.' to reach members of the Kotlin object.",
        )?;
        edits.push(TextEdit::replace(keyword_start, keyword_end, "class"));
        Ok(())
    }

    fn static_class_edit(text: &SourceText<'_>, keyword_start: usize) -> Option<TextEdit> {
        (text.previous_word(keyword_start) == "static").then_some(())?;
        let static_end = text.previous_code(keyword_start).unwrap_or(0) + 1;
        Some(TextEdit::delete(text.word_start(static_end), keyword_start))
    }

    fn verb_methods(text: &SourceText<'_>) -> Vec<(usize, &'static str)> {
        let verbs = Self::VERBS.iter().map(|(verb, _)| *verb);
        let mut methods: Vec<(usize, &'static str)> = verbs
            .chain(Self::RETIRED_VERBS)
            .flat_map(|verb| Self::declarations_of(text, verb))
            .collect();
        methods.sort_unstable();
        methods
    }

    fn declarations_of(text: &SourceText<'_>, verb: &'static str) -> Vec<(usize, &'static str)> {
        text.code_words(verb)
            .into_iter()
            .filter(|at| Self::is_declaration(text, *at, verb.len()))
            .map(|at| (at, verb))
            .collect()
    }

    fn is_declaration(text: &SourceText<'_>, at: usize, length: usize) -> bool {
        let opens = text
            .next_code(at + length)
            .is_some_and(|open| text.bytes[open] == b'(');
        let line_start = text.line_start(at);
        let before = &text.source[line_start..at];
        let previous = text.previous_code(at);
        let after_member =
            previous.is_some_and(|previous| Self::MEMBER_MARKS.contains(&text.bytes[previous]));
        let go_receiver = before.trim_start().starts_with("func (");
        let inside_call = [before.contains('('), go_receiver] == [true, false];
        let is_call = [
            before.contains('='),
            inside_call,
            Self::NOT_DECLARATIONS.contains(&text.previous_word(at)),
        ];
        [opens, after_member, is_call.contains(&true)] == [true, false, false]
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
        let shares_line = before.contains(['{', '}', ';']);
        (!shares_line).then_some(()).ok_or_else(|| {
            self.manual(format!("The {verb} method does not start its own line."))
        })?;
        let name = Self::method_name(extension, verb);
        edits.push(TextEdit::replace(at, at + verb.len(), &name));
        edits.extend(Self::php_request_edit(text, extension, at + verb.len()));
        let handler_edits = self
            .handler
            .as_ref()
            .map(|_| Self::handler_method_edits(text, extension, line_start, at, verb));
        edits.extend(handler_edits.into_iter().flatten());
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

    fn method_name(extension: &str, verb: &str) -> String {
        let lower = verb.to_ascii_lowercase();
        if Self::CAPITALIZED_METHODS.contains(&extension) {
            return format!("{}{}", &verb[..1], &lower[1..]);
        }
        lower
    }

    fn php_request_edit(
        text: &SourceText<'_>,
        extension: &str,
        after_name: usize,
    ) -> Option<TextEdit> {
        (extension == "php").then_some(())?;
        Self::unused_php_request(text, after_name)
    }

    fn unused_php_request(text: &SourceText<'_>, after_name: usize) -> Option<TextEdit> {
        let open = text.next_code(after_name)?;
        (text.bytes[open] == b'(').then_some(())?;
        let close = text.matching(open)?;
        let parameter = text.source[open + 1..close].trim();
        let variable = parameter.strip_prefix('$');
        let is_one_variable = variable.is_some_and(|name| name.bytes().all(Self::is_name_byte));
        let unused = is_one_variable && text.code_words(parameter).len() == 1;
        unused.then(|| TextEdit::delete(open + 1, close))
    }

    fn is_name_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }

    fn handler_method_edits(
        text: &SourceText<'_>,
        extension: &str,
        line_start: usize,
        at: usize,
        verb: &str,
    ) -> Vec<TextEdit> {
        let mut edits = Self::static_removals(text, extension, line_start, at);
        edits.extend(Self::python_receiver(text, extension, at, verb.len()));
        edits
    }

    fn python_receiver(
        text: &SourceText<'_>,
        extension: &str,
        at: usize,
        length: usize,
    ) -> Option<TextEdit> {
        (extension == "py").then_some(())?;
        let open = text.next_code(at + length).unwrap_or(at);
        let empty = text
            .next_code(open + 1)
            .is_some_and(|next| text.bytes[next] == b')');
        let receiver = if empty { "self" } else { "self, " };
        Some(TextEdit::insert(open + 1, receiver))
    }

    fn static_word(extension: &str) -> Option<(&'static str, bool)> {
        match extension {
            "py" => Some(("staticmethod", true)),
            "kt" => Some(("JvmStatic", true)),
            "go" => None,
            _ => Some(("static", false)),
        }
    }

    fn static_removals(
        text: &SourceText<'_>,
        extension: &str,
        line_start: usize,
        at: usize,
    ) -> Vec<TextEdit> {
        let static_word = Self::static_word(extension);
        static_word
            .map(|(word, marked)| {
                let block_start = Self::marker_block_start(text, extension, line_start);
                text.code_words(word)
                    .into_iter()
                    .filter(|found| (block_start..at).contains(found))
                    .map(|found| Self::static_removal(text, line_start, found, word, marked))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn marker_block_start(text: &SourceText<'_>, extension: &str, line_start: usize) -> usize {
        let first = text.previous_line_start(line_start);
        let previous_lines = std::iter::successors(first, |at| text.previous_line_start(*at));
        let marker_lines = previous_lines
            .take_while(|previous| Marker::at_line(text, extension, *previous).is_some());
        marker_lines.last().unwrap_or(line_start)
    }

    fn static_removal(
        text: &SourceText<'_>,
        line_start: usize,
        found: usize,
        word: &str,
        marked: bool,
    ) -> TextEdit {
        let start = found - usize::from(marked);
        let word_end = found + word.len();
        let own_line = text.line_start(found);
        let only_word = text.line_text(own_line).trim() == &text.source[start..word_end];
        if [own_line != line_start, only_word] == [true, true] {
            return TextEdit::delete(own_line, text.line_end(own_line));
        }
        let blanks = text.bytes[word_end..]
            .iter()
            .take_while(|byte| Self::SPACES.contains(byte))
            .count();
        TextEdit::delete(start, word_end + blanks)
    }
}
