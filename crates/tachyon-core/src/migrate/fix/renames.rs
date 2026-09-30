use super::ManualStep;
use super::edits::TextEdit;
use super::scan::SourceText;
use crate::behavior::{RenamedAnnotation, RoutingRename};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct AnnotationRename;

impl AnnotationRename {
    const NAMESPACES: [&str; 2] = ["yon::", "tac::"];

    pub(super) fn plan(relative: &str, source: &str) -> (Vec<TextEdit>, Vec<String>) {
        let extension = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let mut edits = Vec::new();
        let mut notes = Vec::new();
        let Some(text) = SourceText::read(extension, source) else {
            return (edits, notes);
        };
        let prefix = Self::prefix(extension);
        let mut line_start = 0;
        while line_start < text.len() {
            let written_prefix = if extension == "go"
                && text.line_text(line_start).trim_start().starts_with("// @")
            {
                "// @"
            } else {
                prefix
            };
            if let Some((edit, old, record)) = Self::line_edit(&text, written_prefix, line_start) {
                notes.push(format!(
                    "Renamed '{old}' to '{}' on line {} (ADR {record}).",
                    edit.replacement,
                    text.line_number(line_start)
                ));
                edits.push(edit);
            }
            line_start = text.line_end(line_start);
        }
        (edits, notes)
    }

    fn prefix(extension: &str) -> &'static str {
        match extension {
            "rs" | "php" | "go" => "#[",
            "cs" => "[",
            _ => "@",
        }
    }

    fn line_edit(
        text: &SourceText<'_>,
        prefix: &str,
        line_start: usize,
    ) -> Option<(TextEdit, String, &'static str)> {
        let line = text.line_text(line_start);
        let marker_start = line_start + line.len() - line.trim_start().len();
        let code_start = marker_start + usize::from(prefix == "// @") * 3;
        if !text.is_code(code_start) || !line.trim_start().starts_with(prefix) {
            return None;
        }
        let mut name_start = marker_start + prefix.len();
        for namespace in Self::NAMESPACES {
            if text.source[name_start..].starts_with(namespace) {
                name_start += namespace.len();
            }
        }
        let name_end = text.word_end(name_start);
        let name = &text.source[name_start..name_end];
        if name.is_empty() || text.source[name_end..].starts_with('.') {
            return None;
        }
        let (written, record) = match RenamedAnnotation::new_name(name) {
            Some(new) => (RenamedAnnotation::spelled_like(name, new), "0047"),
            None => (
                RoutingRename::spelled_like(name, RoutingRename::direct(name)?),
                "0048",
            ),
        };
        Some((
            TextEdit::replace(name_start, name_end, &written),
            String::from(name),
            record,
        ))
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
        let mut rename = Self::default();
        let mut destinations = BTreeSet::new();
        for relative in folder_files {
            if let Some(target) = Self::folder_target(relative) {
                rename.add_move(root, relative, target, &mut destinations);
            }
        }
        for relative in sources {
            if let Some(target) = Self::test_setup_target(relative) {
                rename.add_move(root, relative, target, &mut destinations);
            }
        }
        rename
    }

    pub(super) fn is_allowed_move(from: &str, to: &str) -> bool {
        if Self::folder_target(from).as_deref() == Some(to) {
            return true;
        }
        Self::test_setup_target(from).as_deref() == Some(to)
    }

    pub(super) fn renamed_folder_files(relative: &str) -> bool {
        let mut parts = relative.split('/');
        let realm = parts.next().unwrap_or("");
        let folder = parts.next().unwrap_or("");
        Self::REALMS.contains(&realm) && Self::FOLDERS.iter().any(|(old, ..)| *old == folder)
    }

    pub(super) fn type_edits(&self, source: &str) -> Vec<TextEdit> {
        let mut edits = Vec::new();
        for (old, new) in &self.type_names {
            for (old_word, new_word) in Self::spellings(old, new) {
                for at in Self::word_positions(source, &old_word) {
                    edits.push(TextEdit::replace(at, at + old_word.len(), &new_word));
                }
            }
        }
        edits
    }

    pub(super) fn path_edits(&self, source: &str) -> Vec<TextEdit> {
        self.folder_paths
            .iter()
            .flat_map(|(old, new)| Self::replacements(source, old, new))
            .collect()
    }

    fn replacements(source: &str, old: &str, new: &str) -> Vec<TextEdit> {
        source
            .match_indices(old)
            .map(|(at, _)| TextEdit::replace(at, at + old.len(), new))
            .collect()
    }

    fn add_folder_path(&mut self, from: &str, to: &str) {
        let old_folder = Self::role_folder(from);
        let new_folder = Self::role_folder(to);
        if old_folder.ends_with("/clients/") {
            self.add_remote_paths(from, to);
            return;
        }
        if old_folder != new_folder {
            self.folder_paths.insert((old_folder, new_folder));
        }
    }

    fn add_remote_paths(&mut self, from: &str, to: &str) {
        self.folder_paths
            .insert((String::from(from), String::from(to)));
        let Some((_, rest)) = from.split_once("/clients/") else {
            return;
        };
        for prefix in ["", "./", "../"] {
            let old = format!("{prefix}clients/{rest}");
            let new = format!("{prefix}remotes/{rest}");
            self.folder_paths.insert((old, new));
        }
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
        let taken = destinations.contains(&target) || root.join(&target).exists();
        if taken {
            self.manual_steps.push(ManualStep::new(
                relative,
                format!("'{target}' already exists, so the file cannot move there (ADR 0047)."),
                format!("Move '{relative}' to '{target}' by hand, or merge the two files."),
            ));
            return;
        }
        destinations.insert(target.clone());
        let old_stem = Self::stem(relative);
        let new_stem = Self::stem(&target);
        if old_stem != new_stem && Self::is_type_name(old_stem) {
            let pair = (String::from(old_stem), String::from(new_stem));
            if !self.type_names.contains(&pair) {
                self.type_names.push(pair);
            }
        }
        self.add_folder_path(relative, &target);
        self.moves.push(FileMove {
            from: String::from(relative),
            to: target,
        });
    }

    fn folder_target(relative: &str) -> Option<String> {
        let mut parts = relative.splitn(3, '/');
        let realm = parts.next()?;
        let folder = parts.next()?;
        let rest = parts.next()?;
        if !Self::REALMS.contains(&realm) {
            return None;
        }
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
        if !Self::REALMS.contains(&realm) || folder != "tests" || rest.contains('/') {
            return None;
        }
        let (old_suffix, new_suffix) = Self::TEST_SUFFIX;
        let renamed = Self::renamed_file(rest, old_suffix, new_suffix);
        if renamed == rest {
            return None;
        }
        Some(format!("{realm}/tests/{renamed}"))
    }

    fn renamed_file(rest: &str, old_suffix: &str, new_suffix: &str) -> String {
        let (directory, file) = rest
            .rsplit_once('/')
            .map_or(("", rest), |(directory, file)| (directory, file));
        let (stem, extension) = file.split_once('.').unwrap_or((file, ""));
        let Some(prefix) = stem.strip_suffix(old_suffix) else {
            return String::from(rest);
        };
        if prefix.is_empty() || !Self::is_type_name(stem) {
            return String::from(rest);
        }
        let mut renamed = format!("{prefix}{new_suffix}");
        if !extension.is_empty() {
            renamed = format!("{renamed}.{extension}");
        }
        if directory.is_empty() {
            return renamed;
        }
        format!("{directory}/{renamed}")
    }

    fn stem(relative: &str) -> &str {
        let file = relative.rsplit('/').next().unwrap_or(relative);
        file.split('.').next().unwrap_or(file)
    }

    fn is_type_name(name: &str) -> bool {
        let starts_upper = name.starts_with(|character: char| character.is_ascii_uppercase());
        starts_upper
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
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
        let bytes = source.as_bytes();
        let is_word = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$');
        let mut positions = Vec::new();
        for (at, _) in source.match_indices(word) {
            let end = at + word.len();
            let starts = at == 0 || !is_word(bytes[at - 1]);
            let ends = end == bytes.len() || !is_word(bytes[end]);
            if starts && ends {
                positions.push(at);
            }
        }
        positions
    }
}

pub(super) struct HiddenAnnotation;

impl HiddenAnnotation {
    pub(super) fn steps(relative: &str, updated: &str) -> Vec<ManualStep> {
        let path = Path::new(relative);
        let is_script = path
            .extension()
            .is_some_and(|extension| extension == "js" || extension == "ts");
        if !is_script {
            return Vec::new();
        }
        let Ok(names) = crate::annotation_layout::declaration_names(path, updated) else {
            return Vec::new();
        };
        let mut steps = Vec::new();
        for name in names {
            let used_as_annotation = updated.lines().any(|line| {
                line.trim_start()
                    .strip_prefix('@')
                    .is_some_and(|rest| Self::names(rest, &name))
            });
            if used_as_annotation && Self::is_new_annotation(&name) {
                steps.push(ManualStep::new(
                    relative,
                    format!("The class '{name}' has the same name as the global annotation @{name}, so TypeScript and editors cannot see the annotation."),
                    format!("Rename the class, for example to 'App{name}', and every use of it."),
                ));
            }
        }
        steps
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
