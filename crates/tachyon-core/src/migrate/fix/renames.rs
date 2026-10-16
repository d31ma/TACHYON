use super::ManualStep;
use super::edits::TextEdit;
use super::markers::Marker;
use super::scan::SourceText;
use super::shared_utils_move::SharedUtilsMove;
use crate::behavior::{RenamedAnnotation, RoutingRename};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct AnnotationRename;

impl AnnotationRename {
    const PREFIXES: [(&str, &str); 4] = [("rs", "#["), ("php", "#["), ("go", "#["), ("cs", "[")];

    pub(super) fn plan(relative: &str, source: &str) -> (Vec<TextEdit>, Vec<String>) {
        let extension = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let text = SourceText::read(extension, source);
        text.map(|text| Self::renames(&text, extension))
            .unwrap_or_default()
    }

    fn renames(text: &SourceText<'_>, extension: &str) -> (Vec<TextEdit>, Vec<String>) {
        text.line_starts(0)
            .into_iter()
            .filter_map(|line_start| Self::line_rename(text, extension, line_start))
            .unzip()
    }

    fn line_rename(
        text: &SourceText<'_>,
        extension: &str,
        line_start: usize,
    ) -> Option<(TextEdit, String)> {
        let prefix = Self::written_prefix(text, extension, line_start);
        let (edit, old, record) = Self::line_edit(text, prefix, line_start)?;
        let note = format!(
            "Renamed '{old}' to '{}' on line {} (ADR {record}).",
            edit.replacement,
            text.line_number(line_start)
        );
        Some((edit, note))
    }

    fn written_prefix(text: &SourceText<'_>, extension: &str, line_start: usize) -> &'static str {
        let go_comment = text.line_text(line_start).trim_start().starts_with("// @");
        if [extension == "go", go_comment] == [true, true] {
            return "// @";
        }
        Self::prefix(extension)
    }

    fn prefix(extension: &str) -> &'static str {
        let known = Self::PREFIXES
            .iter()
            .find(|(written, _)| *written == extension);
        known.map_or("@", |(_, prefix)| *prefix)
    }

    fn line_edit(
        text: &SourceText<'_>,
        prefix: &str,
        line_start: usize,
    ) -> Option<(TextEdit, String, &'static str)> {
        let marker_start = line_start + text.indentation(line_start);
        let code_start = marker_start + usize::from(prefix == "// @") * 3;
        let written_marker = text.line_text(line_start).trim_start().starts_with(prefix);
        ([text.is_code(code_start), written_marker] == [true, true]).then_some(())?;
        let name_start = Marker::name_start(text, marker_start + prefix.len());
        let name_end = text.word_end(name_start);
        let name = &text.source[name_start..name_end];
        let qualified = text.source[name_end..].starts_with('.');
        ([name.is_empty(), qualified] == [false, false]).then_some(())?;
        let (written, record) = Self::renamed(name)?;
        Some((
            TextEdit::replace(name_start, name_end, &written),
            String::from(name),
            record,
        ))
    }

    fn renamed(name: &str) -> Option<(String, &'static str)> {
        let Some(new) = RenamedAnnotation::new_name(name) else {
            let routing = RoutingRename::direct(name)?;
            return Some((RoutingRename::spelled_like(name, routing), "0048"));
        };
        Some((RenamedAnnotation::spelled_like(name, new), "0047"))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FileMove {
    pub(super) from: String,
    pub(super) to: String,
}

impl FileMove {
    pub(super) fn changes_only_case(&self) -> bool {
        self.from.eq_ignore_ascii_case(&self.to)
    }
}

#[derive(Default)]
pub(super) struct RoleRename {
    pub(super) moves: Vec<FileMove>,
    pub(super) type_names: Vec<(String, String)>,
    pub(super) folder_paths: BTreeSet<(String, String)>,
    pub(super) manual_steps: Vec<ManualStep>,
    moves_utilities: bool,
}

impl RoleRename {
    const REALMS: [&str; 2] = ["client", "server"];
    const FOLDERS: [(&str, &str, &str, &str); 5] = [
        ("clients", "remotes", "Client", "Client"),
        ("repositories", "stores", "Repository", "Store"),
        ("configurations", "setups", "Configuration", "Setup"),
        ("mappers", "converters", "Mapper", "Converter"),
        ("delegates", "helpers", "Component", "Helper"),
    ];
    const TEST_SUFFIX: (&str, &str) = ("TestConfiguration", "TestSetup");

    pub(super) fn plan(root: &Path, folder_files: &[String], sources: &[String]) -> Self {
        let folder_moves = folder_files
            .iter()
            .filter_map(|relative| Self::folder_target(relative).map(|target| (relative, target)));
        let test_moves = sources.iter().filter_map(|relative| {
            Self::test_setup_target(relative).map(|target| (relative, target))
        });
        let mut rename = Self::default();
        rename.add_moves(root, folder_moves.chain(test_moves));
        rename
    }

    fn add_moves<'a>(&mut self, root: &Path, moves: impl Iterator<Item = (&'a String, String)>) {
        let mut destinations = BTreeSet::new();
        for (relative, target) in moves {
            self.add_move(root, relative, target, &mut destinations);
        }
    }

    pub(super) fn is_allowed_move(from: &str, to: &str) -> bool {
        let folder_move = Self::folder_target(from).as_deref() == Some(to);
        let test_move = Self::test_setup_target(from).as_deref() == Some(to);
        [folder_move, test_move].contains(&true)
    }

    pub(super) fn renamed_folder_files(relative: &str) -> bool {
        let mut parts = relative.split('/');
        let realm = parts.next().unwrap_or("");
        let folder = parts.next().unwrap_or("");
        let in_realm = Self::REALMS.contains(&realm);
        let renamed_folder = Self::FOLDERS.iter().any(|(old, ..)| *old == folder);
        let renamed = [in_realm, renamed_folder] == [true, true];
        [renamed, SharedUtilsMove::moves_folder(relative)].contains(&true)
    }

    pub(super) fn type_edits(&self, source: &str) -> Vec<TextEdit> {
        self.type_names
            .iter()
            .flat_map(|(old, new)| Self::spellings(old, new))
            .flat_map(|(old_word, new_word)| Self::word_edits(source, &old_word, &new_word))
            .collect()
    }

    fn word_edits(source: &str, old_word: &str, new_word: &str) -> Vec<TextEdit> {
        Self::word_positions(source, old_word)
            .into_iter()
            .map(|at| TextEdit::replace(at, at + old_word.len(), new_word))
            .collect()
    }

    pub(super) fn path_edits(&self, relative: &str, source: &str) -> Vec<TextEdit> {
        let mut edits: Vec<TextEdit> = self
            .folder_paths
            .iter()
            .flat_map(|(old, new)| Self::replacements(source, old, new))
            .collect();
        let utility_edits = self
            .moves_utilities
            .then(|| SharedUtilsMove::path_edits(relative, source));
        edits.extend(utility_edits.unwrap_or_default());
        edits
    }

    fn replacements(source: &str, old: &str, new: &str) -> Vec<TextEdit> {
        source
            .match_indices(old)
            .map(|(at, _)| TextEdit::replace(at, at + old.len(), new))
            .collect()
    }

    fn add_folder_path(&mut self, from: &str, to: &str) {
        let moves_utilities = SharedUtilsMove::moves_folder(from);
        self.moves_utilities |= moves_utilities;
        let pairs = (!moves_utilities).then(|| Self::folder_path_pairs(from, to));
        self.folder_paths.extend(pairs.unwrap_or_default());
    }

    fn folder_path_pairs(from: &str, to: &str) -> Vec<(String, String)> {
        let old_folder = Self::role_folder(from);
        let new_folder = Self::role_folder(to);
        if old_folder.ends_with("/clients/") {
            return Self::remote_paths(from, to);
        }
        let changed = (old_folder != new_folder).then_some((old_folder, new_folder));
        changed.into_iter().collect()
    }

    fn remote_paths(from: &str, to: &str) -> Vec<(String, String)> {
        let mut pairs = vec![(String::from(from), String::from(to))];
        let rest = from.splitn(3, '/').nth(2).unwrap_or("");
        for prefix in ["", "./", "../"] {
            let old = format!("{prefix}clients/{rest}");
            let new = format!("{prefix}remotes/{rest}");
            pairs.push((old, new));
        }
        pairs
    }

    fn role_folder(relative: &str) -> String {
        let mut parts = relative.splitn(3, '/');
        let realm = parts.next().unwrap_or("");
        let folder = parts.next().unwrap_or("");
        format!("{realm}/{folder}/")
    }

    fn add_move(
        &mut self,
        root: &Path,
        relative: &str,
        target: String,
        destinations: &mut BTreeSet<String>,
    ) {
        let claimed = destinations.contains(&target);
        let taken = [claimed, root.join(&target).exists()].contains(&true);
        if taken {
            self.manual_steps.push(Self::taken_step(relative, &target));
            return;
        }
        destinations.insert(target.clone());
        self.add_type_name(relative, &target);
        self.add_folder_path(relative, &target);
        self.moves.push(FileMove {
            from: String::from(relative),
            to: target,
        });
    }

    fn taken_step(relative: &str, target: &str) -> ManualStep {
        ManualStep::new(
            relative,
            format!("'{target}' already exists, so the file cannot move there (ADR 0047)."),
            format!("Move '{relative}' to '{target}' by hand, or merge the two files."),
        )
    }

    fn add_type_name(&mut self, relative: &str, target: &str) {
        let old_stem = Self::stem(relative);
        let new_stem = Self::stem(target);
        let renames_type = [old_stem != new_stem, Self::is_type_name(old_stem)] == [true, true];
        let pair = (String::from(old_stem), String::from(new_stem));
        let new_pair = [renames_type, !self.type_names.contains(&pair)] == [true, true];
        self.type_names.extend(new_pair.then_some(pair));
    }

    fn folder_target(relative: &str) -> Option<String> {
        let shared_utilities = SharedUtilsMove::target(relative);
        shared_utilities.or_else(|| Self::role_folder_target(relative))
    }

    fn role_folder_target(relative: &str) -> Option<String> {
        let mut parts = relative.splitn(3, '/');
        let realm = parts.next()?;
        let folder = parts.next()?;
        let rest = parts.next()?;
        Self::REALMS.contains(&realm).then_some(())?;
        let (_, new_folder, old_suffix, new_suffix) =
            Self::FOLDERS.iter().find(|(old, ..)| *old == folder)?;
        let renamed = Self::renamed_file(rest, old_suffix, new_suffix);
        Some(format!("{realm}/{new_folder}/{renamed}"))
    }

    fn test_setup_target(relative: &str) -> Option<String> {
        let mut parts = relative.splitn(3, '/');
        let realm = parts.next()?;
        let folder = parts.next()?;
        let rest = parts.next()?;
        let checks = [
            Self::REALMS.contains(&realm),
            folder == "tests",
            !rest.contains('/'),
        ];
        (checks == [true; 3]).then_some(())?;
        let (old_suffix, new_suffix) = Self::TEST_SUFFIX;
        let renamed = Self::renamed_file(rest, old_suffix, new_suffix);
        (renamed != rest).then(|| format!("{realm}/tests/{renamed}"))
    }

    fn renamed_file(rest: &str, old_suffix: &str, new_suffix: &str) -> String {
        let renamed = Self::renamed_path(rest, old_suffix, new_suffix);
        renamed.unwrap_or_else(|| String::from(rest))
    }

    fn renamed_path(rest: &str, old_suffix: &str, new_suffix: &str) -> Option<String> {
        let (directory, file) = rest
            .rsplit_once('/')
            .map_or(("", rest), |(directory, file)| (directory, file));
        let (stem, extension) = file.split_once('.').unwrap_or((file, ""));
        let prefix = stem.strip_suffix(old_suffix)?;
        ([prefix.is_empty(), Self::is_type_name(stem)] == [false, true]).then_some(())?;
        let renamed = Self::with_extension(format!("{prefix}{new_suffix}"), extension);
        Some(Self::in_directory(directory, renamed))
    }

    fn with_extension(name: String, extension: &str) -> String {
        if extension.is_empty() {
            return name;
        }
        format!("{name}.{extension}")
    }

    fn in_directory(directory: &str, name: String) -> String {
        if directory.is_empty() {
            return name;
        }
        format!("{directory}/{name}")
    }

    fn stem(relative: &str) -> &str {
        let file = relative.rsplit('/').next().unwrap_or(relative);
        file.split('.').next().unwrap_or(file)
    }

    fn is_type_name(name: &str) -> bool {
        let starts_upper = name.starts_with(|character: char| character.is_ascii_uppercase());
        let type_bytes = name.bytes().all(Self::is_type_byte);
        [starts_upper, type_bytes] == [true, true]
    }

    fn is_type_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }

    fn is_word_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_', byte == b'$'].contains(&true)
    }

    fn spellings(old: &str, new: &str) -> Vec<(String, String)> {
        vec![
            (String::from(old), String::from(new)),
            (Self::lower_camel(old), Self::lower_camel(new)),
            (
                RenamedAnnotation::snake_case(old),
                RenamedAnnotation::snake_case(new),
            ),
        ]
    }

    fn lower_camel(name: &str) -> String {
        let mut characters = name.chars();
        let first = characters.next().unwrap_or_default().to_ascii_lowercase();
        format!("{first}{}", characters.as_str())
    }

    pub(super) fn word_positions(source: &str, word: &str) -> Vec<usize> {
        source
            .match_indices(word)
            .map(|(at, _)| at)
            .filter(|at| Self::is_whole_word(source.as_bytes(), *at, word.len()))
            .collect()
    }

    fn is_whole_word(bytes: &[u8], at: usize, length: usize) -> bool {
        let before = at.checked_sub(1).map(|before| bytes[before]);
        let after = bytes.get(at + length).copied();
        let word_before = before.is_some_and(Self::is_word_byte);
        let word_after = after.is_some_and(Self::is_word_byte);
        [word_before, word_after] == [false, false]
    }
}

pub(super) struct HiddenAnnotation;

impl HiddenAnnotation {
    const SCRIPT_EXTENSIONS: [&str; 2] = ["js", "ts"];

    pub(super) fn steps(relative: &str, updated: &str) -> Vec<ManualStep> {
        Self::script_declarations(relative, updated)
            .into_iter()
            .filter(|name| Self::hides_annotation(updated, name))
            .map(|name| Self::rename_step(relative, &name))
            .collect()
    }

    fn script_declarations(relative: &str, updated: &str) -> Vec<String> {
        let path = Path::new(relative);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !Self::SCRIPT_EXTENSIONS.contains(&extension) {
            return Vec::new();
        }
        crate::annotation_layout::declaration_names(path, updated).unwrap_or_default()
    }

    fn hides_annotation(updated: &str, name: &str) -> bool {
        let used_as_annotation = updated.lines().any(|line| Self::annotates(line, name));
        [used_as_annotation, Self::is_new_annotation(name)] == [true, true]
    }

    fn annotates(line: &str, name: &str) -> bool {
        let annotation = line.trim_start().strip_prefix('@');
        annotation.is_some_and(|rest| Self::names(rest, name))
    }

    fn rename_step(relative: &str, name: &str) -> ManualStep {
        ManualStep::new(
            relative,
            format!(
                "The class '{name}' has the same name as the global annotation @{name}, so TypeScript and editors cannot see the annotation."
            ),
            format!("Rename the class, for example to 'App{name}', and every use of it."),
        )
    }

    fn names(rest: &str, name: &str) -> bool {
        rest.strip_prefix(name).is_some_and(|after| {
            !after.starts_with(|character: char| {
                character.is_ascii_alphanumeric() || character == '_'
            })
        })
    }

    fn is_new_annotation(name: &str) -> bool {
        RenamedAnnotation::NAMES.iter().any(|(_, new)| *new == name)
    }
}
