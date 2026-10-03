use super::{
    MigrationAnalysis, is_ignored_directory, migration_failure, portable_path, sorted_entries,
};
use crate::Failure;
use braced_attributes::BracedAttributeEdits;
use clients::ClientConversion;
use edits::{Hunk, TextEdit};
use file_name_fix::FileNameFix;
use imports::ImportRemoval;
use inline_model_fix::InlineModelFix;
use manifest::ManifestMigration;
use middleware::{MiddlewareConversion, MiddlewarePlan};
use model_split_fix::ModelSplitFix;
use one_class::OneClassReport;
use removed_control_edits::RemovedControlEdits;
use renames::{AnnotationRename, FileMove, HiddenAnnotation, RoleRename};
use routes::RouteMigration;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path};

mod arguments;
mod braced_attributes;
mod clients;
mod edits;
mod events;
mod field_rename;
mod field_use;
mod file_name_fix;
mod go_markers;
mod imports;
mod inline_model;
mod inline_model_fix;
mod inline_models;
mod known_model;
mod manifest;
mod markers;
mod member_reference;
mod middleware;
mod model_importers;
mod model_split;
mod model_split_fix;
mod model_text;
mod native_http;
mod needs_all;
mod nullable_field_edits;
mod one_class;
mod private_keyword;
mod private_members;
mod private_rename;
mod python_parameters;
mod realm_models;
mod removed_control_edits;
mod renames;
mod routes;
mod scan;
mod shared_utils_move;
mod simpler_forms_fix;
mod stored_fields;
mod streams;
mod type_comments_fix;
mod type_import;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod rename_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod route_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod shared_utils_move_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod field_rename_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod manifest_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod argument_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod event_tests;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod stream_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod native_http_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod file_name_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod model_split_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod inline_model_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod private_member_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod type_comment_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod visibility_off;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileChange {
    source: String,
    notes: Vec<String>,
    hunks: Vec<Hunk>,
    original: String,
    updated: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ManualStep {
    source: String,
    problem: String,
    action: String,
}

impl ManualStep {
    fn new(source: &str, problem: String, action: String) -> Self {
        Self {
            source: String::from(source),
            problem,
            action,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MigrationFixPlan {
    changes: Vec<FileChange>,
    moves: Vec<FileMove>,
    deletes: Vec<String>,
    legacy_identity: Option<String>,
    manual_steps: Vec<ManualStep>,
    unused_members: Vec<crate::annotation_layout::UnusedMember>,
    missing_types: Vec<crate::annotation_layout::MissingType>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MigrationFix;

impl MigrationFix {
    const LANGUAGES: [&str; 12] = [
        "js", "ts", "py", "php", "java", "kt", "cs", "rs", "swift", "dart", "go", "rb",
    ];
    const SOURCE_FOLDERS: [&str; 2] = ["client", "server"];

    pub fn plan(project_root: impl AsRef<Path>) -> Result<MigrationFixPlan, Failure> {
        let root = project_root.as_ref();
        super::require_directory(root)?;
        let mut plan = MigrationFixPlan::default();
        let sources = Self::project_files(root)?;
        let folder_files = Self::role_folder_files(root, &mut plan.manual_steps)?;
        let roles = RoleRename::plan(root, &folder_files, &sources);
        plan.manual_steps.extend(roles.manual_steps.iter().cloned());
        let routes = RouteMigration::plan(root)?;
        plan.manual_steps
            .extend(routes.manual_steps.iter().cloned());
        plan.unused_members = crate::annotation_layout::DesignCheck::unused_members(root);
        plan.missing_types = crate::annotation_layout::DesignCheck::missing_types(root);
        let mut route_moves = Vec::new();
        for relative in &sources {
            route_moves.extend(plan.add_file(root, relative, &roles, &routes));
        }
        for file in &routes.files {
            if !sources.contains(&file.from) {
                plan.manual_steps.push(ManualStep::new(
                    &file.from,
                    String::from("This route file is too large for the fix to read (ADR 0048)."),
                    format!("Move it to '{}' by hand and make it a controller.", file.to),
                ));
            }
        }
        plan.moves.clone_from(&roles.moves);
        plan.moves.extend(route_moves);
        plan.deletes.clone_from(&routes.deletes);
        plan.add_file_names(root, &sources);
        plan.add_stored_views(root)?;
        plan.add_manifest(ManifestMigration::plan(root));
        Ok(plan)
    }

    fn role_folder_files(
        root: &Path,
        manual_steps: &mut Vec<ManualStep>,
    ) -> Result<Vec<String>, Failure> {
        let mut files = Vec::new();
        let mut pending = Vec::new();
        for folder in Self::SOURCE_FOLDERS {
            let directory = root.join(folder);
            if !Self::is_real_directory(&directory) {
                continue;
            }
            for entry in sorted_entries(&directory)? {
                let relative = format!("{folder}/{}", entry.file_name().to_string_lossy());
                if RoleRename::renamed_folder_files(&format!("{relative}/")) {
                    pending.push(entry.path());
                }
            }
        }
        while let Some(directory) = pending.pop() {
            if !Self::is_real_directory(&directory) {
                let relative = portable_path(directory.strip_prefix(root).unwrap_or(&directory));
                manual_steps.push(ManualStep::new(
                    &relative,
                    String::from("A renamed role folder is a link, so the fix does not follow it (ADR 0047)."),
                    String::from("Replace the link with a real folder under its new name."),
                ));
                continue;
            }
            for entry in sorted_entries(&directory)? {
                let path = entry.path();
                let metadata = Self::metadata(&path)?;
                let relative = portable_path(path.strip_prefix(root).unwrap_or(&path));
                if metadata.is_dir() && !metadata.is_symlink() {
                    pending.push(path);
                } else if metadata.is_file() && !metadata.is_symlink() {
                    files.push(relative);
                } else {
                    manual_steps.push(ManualStep::new(
                        &relative,
                        String::from("This link sits in a renamed role folder, so the fix does not move it (ADR 0047)."),
                        String::from("Move the link or its target by hand."),
                    ));
                }
            }
        }
        files.sort();
        Ok(files)
    }

    fn is_real_directory(path: &Path) -> bool {
        fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir() && !metadata.is_symlink())
    }

    fn project_files(root: &Path) -> Result<Vec<String>, Failure> {
        let mut files = Vec::new();
        for entry in sorted_entries(root)? {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if name.starts_with("middleware.")
                && Self::is_source(&path)
                && Self::is_small_file(&path)?
            {
                files.push(name);
            }
        }
        for folder in Self::SOURCE_FOLDERS {
            let directory = root.join(folder);
            let is_directory =
                fs::symlink_metadata(&directory).is_ok_and(|metadata| metadata.is_dir());
            if is_directory {
                Self::collect_sources(root, &directory, &mut files)?;
            }
        }
        files.sort();
        Ok(files)
    }

    fn collect_sources(root: &Path, folder: &Path, files: &mut Vec<String>) -> Result<(), Failure> {
        let mut pending = vec![folder.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in sorted_entries(&directory)? {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                let metadata = Self::metadata(&path)?;
                if metadata.is_dir() && !metadata.is_symlink() && !is_ignored_directory(&name) {
                    pending.push(path);
                    continue;
                }
                if Self::is_source(&path) && Self::is_small_file(&path)? {
                    let relative = path.strip_prefix(root).unwrap_or(&path);
                    files.push(portable_path(relative));
                }
            }
        }
        Ok(())
    }

    fn is_writable_target(relative: &str) -> bool {
        if relative == ManifestMigration::MANIFEST {
            return true;
        }
        let path = Path::new(relative);
        for part in path.components() {
            if !matches!(part, Component::Normal(_)) {
                return false;
            }
        }
        let extension = path.extension().and_then(|value| value.to_str());
        let is_root_middleware = !relative.contains('/') && relative.starts_with("middleware.");
        if is_root_middleware {
            return Self::is_source(path);
        }
        let in_source_folder = Self::SOURCE_FOLDERS
            .iter()
            .any(|folder| relative.starts_with(&format!("{folder}/")));
        let is_client_view = relative.starts_with("client/") && relative.ends_with("/tac.html");
        is_client_view
            || (in_source_folder
                && extension.is_some_and(|extension| Self::LANGUAGES.contains(&extension)))
    }

    fn is_allowed_move(file_move: &FileMove) -> bool {
        let normal = |relative: &str| {
            Path::new(relative)
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
        };
        normal(&file_move.from)
            && normal(&file_move.to)
            && (RoleRename::is_allowed_move(&file_move.from, &file_move.to)
                || RouteMigration::is_allowed_move(&file_move.from, &file_move.to)
                || FileNameFix::is_allowed_move(&file_move.from, &file_move.to))
    }

    fn is_source(path: &Path) -> bool {
        let extension = path.extension().and_then(|value| value.to_str());
        extension.is_some_and(|extension| Self::LANGUAGES.contains(&extension))
    }

    fn is_small_file(path: &Path) -> Result<bool, Failure> {
        let metadata = Self::metadata(path)?;
        let is_file = metadata.is_file() && !metadata.is_symlink();
        Ok(is_file && metadata.len() <= MigrationAnalysis::MAX_SOURCE_BYTES)
    }

    fn metadata(path: &Path) -> Result<fs::Metadata, Failure> {
        fs::symlink_metadata(path).map_err(|error| {
            migration_failure(&format!("Cannot inspect '{}': {error}", path.display()))
        })
    }
}

impl MigrationFixPlan {
    #[must_use]
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }

    #[must_use]
    pub fn manual_step_count(&self) -> usize {
        self.manual_steps.len()
    }

    #[must_use]
    pub fn move_count(&self) -> usize {
        self.moves.len()
    }

    #[must_use]
    pub fn delete_count(&self) -> usize {
        self.deletes.len() + usize::from(self.legacy_identity.is_some())
    }

    fn add_file_names(&mut self, root: &Path, sources: &[String]) {
        let mut taken: Vec<String> = self.moves.iter().map(|moved| moved.from.clone()).collect();
        taken.extend(self.changes.iter().map(|change| change.source.clone()));
        let retired = sources
            .iter()
            .filter(|relative| relative.starts_with(RouteMigration::ROOT));
        taken.extend(retired.cloned());
        let texts: Vec<(String, String)> = sources
            .iter()
            .filter_map(|relative| {
                let text = fs::read_to_string(root.join(relative)).ok()?;
                Some((relative.clone(), text))
            })
            .collect();
        let split = ModelSplitFix::plan(&texts, &taken);
        taken.extend(split.changes.iter().map(|change| change.source.clone()));
        let inline = InlineModelFix::plan(&texts, &taken);
        taken.extend(inline.changes.iter().map(|change| change.source.clone()));
        let created = [split.created_files(), inline.created_files()].concat();
        self.changes.extend(split.changes);
        self.manual_steps.extend(split.manual_steps);
        self.changes.extend(inline.changes);
        self.manual_steps.extend(inline.manual_steps);
        let fix = FileNameFix::plan(&texts, &taken);
        self.moves.extend(
            fix.moves
                .into_iter()
                .filter(|moved| !created.contains(&moved.to.to_lowercase())),
        );
        self.changes.extend(fix.changes);
        self.manual_steps.extend(fix.manual_steps);
    }

    fn add_manifest(&mut self, migration: ManifestMigration) {
        self.manual_steps.extend(migration.manual_steps);
        self.legacy_identity = migration.legacy_identity;
        if migration.edits.is_empty() {
            return;
        }
        let original = migration.original;
        self.changes.push(FileChange {
            source: String::from(ManifestMigration::MANIFEST),
            notes: migration.notes,
            hunks: Hunk::from_edits(&original, &migration.edits),
            updated: TextEdit::apply(&original, &migration.edits),
            original,
        });
    }

    fn add_file(
        &mut self,
        root: &Path,
        relative: &str,
        roles: &RoleRename,
        routes: &RouteMigration,
    ) -> Option<FileMove> {
        let Ok(source) = fs::read_to_string(root.join(relative)) else {
            if routes.file(relative).is_some() {
                self.manual_steps.push(ManualStep::new(
                    relative,
                    String::from("This controller could not be read as UTF-8, so the fix leaves it unchanged."),
                    String::from("Save the controller as readable UTF-8, then run 'ty migrate fix' again."),
                ));
            }
            return None;
        };
        if Path::new(relative)
            .extension()
            .is_some_and(|extension| extension == "rb")
        {
            self.add_ruby_file(root, relative, source);
            return None;
        }
        let removal = ImportRemoval::plan(relative, &source);
        let mut edits = removal.edits;
        let mut notes = removal.notes;
        self.manual_steps.extend(removal.manual_steps);
        let before_middleware = edits.len();
        let middleware_is_manual = self.add_middleware(relative, &source, &mut edits, &mut notes);
        let middleware_converted = edits.len() > before_middleware;
        self.add_native_http(
            relative,
            &source,
            middleware_converted,
            &mut edits,
            &mut notes,
        );
        Self::add_renames(relative, &source, roles, &mut edits, &mut notes);
        self.add_clients(relative, &source, &mut edits, &mut notes);
        self.add_contracts(relative, &source, &mut edits, &mut notes);
        self.add_arguments(relative, &source, &mut edits, &mut notes);
        self.add_events(root, relative, &source, &mut edits, &mut notes);
        self.add_streams(relative, &source, &mut edits, &mut notes);
        let route_move = self.add_route(relative, &source, routes, &mut edits, &mut notes);
        self.add_go_markers(relative, &source, &mut edits, &mut notes);
        Self::add_python_parameters(relative, &source, &mut edits, &mut notes);
        Self::add_needs_all(relative, &source, &mut edits, &mut notes);
        Self::add_nullable_fields(relative, &source, &mut edits, &mut notes);
        Self::add_injected_fields(relative, &source, &mut edits, &mut notes);
        self.add_private_members(relative, &source, &mut edits, &mut notes);
        self.add_stored_fields(relative, &source, &mut edits, &mut notes);
        Self::add_simpler_forms(root, relative, &source, &mut edits, &mut notes);
        self.add_type_comments(relative, &mut edits, &mut notes);
        let updated = TextEdit::apply(&source, &edits);
        let checked = route_move
            .as_ref()
            .or_else(|| {
                roles
                    .moves
                    .iter()
                    .find(|file_move| file_move.from == relative)
            })
            .map_or(relative, |file_move| file_move.to.as_str());
        let retired = route_move.is_none() && relative.starts_with(RouteMigration::ROOT);
        if !retired {
            self.manual_steps
                .extend(HiddenAnnotation::steps(checked, &updated));
            self.manual_steps.extend(OneClassReport::steps(
                checked,
                &updated,
                middleware_is_manual,
            ));
        }
        if edits.is_empty() {
            return route_move;
        }
        self.changes.push(FileChange {
            source: String::from(relative),
            notes,
            hunks: Hunk::from_edits(&source, &edits),
            original: source,
            updated,
        });
        route_move
    }

    fn add_ruby_file(&mut self, root: &Path, relative: &str, source: String) {
        let mut edits = Vec::new();
        let mut notes = Vec::new();
        self.add_events(root, relative, &source, &mut edits, &mut notes);
        self.add_streams(relative, &source, &mut edits, &mut notes);
        Self::add_injected_fields(relative, &source, &mut edits, &mut notes);
        self.add_private_members(relative, &source, &mut edits, &mut notes);
        Self::add_simpler_forms(root, relative, &source, &mut edits, &mut notes);
        self.add_type_comments(relative, &mut edits, &mut notes);
        if edits.is_empty() {
            return;
        }
        self.changes.push(FileChange {
            source: String::from(relative),
            notes,
            hunks: Hunk::from_edits(&source, &edits),
            updated: TextEdit::apply(&source, &edits),
            original: source,
        });
    }

    fn add_clients(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        match ClientConversion::plan(relative, source) {
            Ok((conversion, conversion_notes)) => {
                if conversion.iter().all(|edit| Self::fits(edits, edit)) {
                    edits.extend(conversion);
                    notes.extend(conversion_notes);
                } else {
                    self.manual_steps.push(Self::overlap(relative));
                }
            }
            Err(step) => self.manual_steps.push(step),
        }
    }

    fn add_contracts(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let planned = match crate::declared_contract::DeclaredContract::rewrite(
            Path::new(relative),
            source,
        ) {
            Ok(planned) => planned,
            Err(problem) => {
                self.manual_steps.push(ManualStep::new(
                    relative,
                    problem,
                    String::from("Declare the HTTPClient or Store as a contract without bodies (TY1308), then run 'ty migrate fix --write' again."),
                ));
                return;
            }
        };
        let contract: Vec<TextEdit> = planned
            .iter()
            .filter(|(range, text)| !range.is_empty() || !text.is_empty())
            .map(|(range, text)| TextEdit::replace(range.start, range.end, text))
            .collect();
        if contract.is_empty() {
            return;
        }
        if !Self::all_fit(edits, &contract) {
            self.manual_steps.push(Self::overlap(relative));
            return;
        }
        edits.extend(contract);
        notes.push(String::from(
            "Declared the HTTPClient or Store as a contract without an implementation: Tachyon generates each request and query (TY1308). Inject a Store with Needs instead of building it.",
        ));
    }

    fn add_arguments(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let planned = arguments::ArgumentConversion::plan(relative, source);
        self.manual_steps.extend(planned.steps);
        if planned.edits.is_empty() {
            return;
        }
        if !planned.edits.iter().all(|edit| Self::fits(edits, edit)) {
            self.manual_steps.push(Self::overlap(relative));
            return;
        }
        edits.extend(planned.edits);
        notes.extend(planned.notes);
    }

    fn add_events(
        &mut self,
        root: &Path,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let planned = events::EventConversion::plan(root, relative, source);
        self.manual_steps.extend(planned.steps);
        if planned.edits.is_empty() {
            return;
        }
        if !planned.edits.iter().all(|edit| Self::fits(edits, edit)) {
            self.manual_steps.push(Self::overlap(relative));
            return;
        }
        edits.extend(planned.edits);
        notes.extend(planned.notes);
    }

    fn add_streams(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let planned = streams::StreamConversion::plan(relative, source);
        self.manual_steps.extend(planned.steps);
        if planned.edits.is_empty() {
            return;
        }
        if !planned.edits.iter().all(|edit| Self::fits(edits, edit)) {
            self.manual_steps.push(Self::overlap(relative));
            return;
        }
        edits.extend(planned.edits);
        notes.extend(planned.notes);
    }

    fn add_go_markers(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        if std::path::Path::new(relative)
            .extension()
            .is_none_or(|value| value != "go")
        {
            return;
        }
        let Some(text) = scan::SourceText::read("go", source) else {
            return;
        };
        for (start, _) in source.match_indices("#[") {
            if !text.is_code(start) {
                continue;
            }
            let Some(end) = text.matching(start + 1) else {
                continue;
            };
            let line = text.line_number(start);
            let Some(planned) = go_markers::GoMarkerEdits::plan(&text, start, end) else {
                self.manual_steps.push(ManualStep::new(
                    relative,
                    format!("The Go annotation on line {line} has a comment or a multi-line text value inside it, so it cannot become one `// @Name(...)` line."),
                    String::from("Write it as `// @Name(...)` on one line (ADR 0050), then run 'ty migrate fix' again."),
                ));
                continue;
            };
            if planned.iter().all(|edit| Self::fits(edits, edit)) {
                edits.extend(planned);
                notes.push(format!(
                    "Changed Go annotation to // @ on line {line} (ADR 0050)."
                ));
            }
        }
    }

    fn add_native_http(
        &mut self,
        relative: &str,
        source: &str,
        middleware_converted: bool,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let Some(extension) = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
        else {
            return;
        };
        let Some(text) = scan::SourceText::read(extension, source) else {
            return;
        };
        let mut plan = native_http::NativeHttpFix::plan(extension, &text, middleware_converted);
        for (edit, note) in plan.edits.into_iter().zip(plan.notes) {
            if Self::fits(edits, &edit) {
                edits.push(edit);
                notes.push(note);
            } else if plan.manual.is_none() {
                plan.manual = Some(native_http::NativeHttpFix::manual_step_for(extension));
            }
        }
        if let Some((problem, action)) = plan.manual {
            self.manual_steps
                .push(ManualStep::new(relative, problem, action));
        }
    }

    fn add_needs_all(
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let Some(extension) = std::path::Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
        else {
            return;
        };
        if !crate::annotation_rules::AnnotationRules::LANGUAGES.contains(&extension) {
            return;
        }
        let Some(text) = scan::SourceText::read(extension, source) else {
            return;
        };
        for (at, edit) in needs_all::NeedsAllEdits::plan(&text) {
            if Self::fits(edits, &edit) {
                let line = text.line_number(at);
                edits.push(edit);
                notes.push(format!(
                    "Removed the NeedsAll field list on line {line}: NeedsAll fills every field (ADR 0056)."
                ));
            }
        }
    }

    fn add_nullable_fields(
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let (planned, explained): (Vec<_>, Vec<_>) =
            nullable_field_edits::NullableFieldEdits::plan(relative, source)
                .into_iter()
                .filter(|(edit, _)| Self::fits(edits, edit))
                .unzip();
        edits.extend(planned);
        notes.extend(explained);
    }

    fn add_stored_fields(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        if !relative.starts_with("client/") {
            return;
        }
        let planned = stored_fields::StoredFieldEdits::plan(relative, source);
        for written in planned.clashes {
            let bare = written.trim_start_matches('$');
            self.manual_steps.push(ManualStep::new(
                relative,
                format!("The field `{written}` keeps its value through a `$` prefix, and the class already uses the name `{bare}` (ADR 0059)."),
                format!("Rename one of them, then write @{} on the field.", crate::behavior::StoredState::marker_for(&written)),
            ));
        }
        if planned.edits.iter().all(|edit| Self::fits(edits, edit)) {
            edits.extend(planned.edits);
            notes.extend(planned.notes);
        }
    }

    fn add_injected_fields(
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let renames = field_rename::FieldRename::plan(relative, source)
            .into_iter()
            .filter(|rename| Self::all_fit(edits, &rename.edits))
            .collect::<Vec<_>>();
        for rename in renames {
            edits.extend(rename.edits);
            notes.push(rename.note);
        }
    }

    fn add_private_members(
        &self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let planned = private_members::PrivateMembers::plan(relative, source, &self.unused_members);
        for (member_edits, note) in planned {
            if Self::all_fit(edits, &member_edits) {
                edits.extend(member_edits);
                notes.push(note);
            }
        }
    }

    fn add_simpler_forms(
        root: &Path,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let (fitting, fitting_notes): (Vec<TextEdit>, Vec<String>) =
            simpler_forms_fix::SimplerFormsFix::plan(root, relative, source)
                .into_iter()
                .filter(|(edit, _)| Self::fits(edits, edit))
                .unzip();
        edits.extend(fitting);
        notes.extend(fitting_notes);
    }

    fn add_type_comments(
        &mut self,
        relative: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let planned = type_comments_fix::TypeCommentsFix::edits(relative, &self.missing_types);
        for (edit, note) in planned {
            if Self::fits(edits, &edit) {
                edits.push(edit);
                notes.push(note);
            }
        }
        let steps = type_comments_fix::TypeCommentsFix::steps(relative, &self.missing_types);
        for (problem, action) in steps {
            self.manual_steps
                .push(ManualStep::new(relative, problem, action));
        }
    }

    fn add_view_bindings(
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
        planned: Vec<TextEdit>,
    ) {
        let fits = Self::all_fit(edits, &planned);
        if [fits, planned.is_empty()] == [true, false] {
            edits.extend(planned);
            notes.push(String::from(
                "The view reads each renamed injected field by its new name (TY1332).",
            ));
        }
    }

    fn add_removed_controls(edits: &mut Vec<TextEdit>, notes: &mut Vec<String>, source: &str) {
        let planned = RemovedControlEdits::plan_view(source).edits;
        let fits = Self::all_fit(edits, &planned);
        if [fits, planned.is_empty()] == [true, false] {
            edits.extend(planned);
            notes.push(String::from(
                "The removed <if>, <else> and <for> elements become <logic :if>, <logic else> and <loop :for> (TY1367).",
            ));
        }
    }

    fn all_fit(edits: &[TextEdit], planned: &[TextEdit]) -> bool {
        planned.iter().all(|edit| Self::fits(edits, edit))
    }

    fn add_stored_views(&mut self, root: &Path) -> Result<(), Failure> {
        let bindings = field_rename::FieldRename::view_bindings(root);
        let mut pending = vec![root.join("client")];
        while let Some(directory) = pending.pop() {
            if !MigrationFix::is_real_directory(&directory) {
                continue;
            }
            for entry in sorted_entries(&directory)? {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                let metadata = MigrationFix::metadata(&path)?;
                if metadata.is_dir() && !metadata.is_symlink() && !is_ignored_directory(&name) {
                    pending.push(path);
                } else if name == "tac.html" && MigrationFix::is_small_file(&path)? {
                    let Ok(source) = fs::read_to_string(&path) else {
                        continue;
                    };
                    let mut edits = stored_fields::StoredFieldEdits::plan_view(&source);
                    let mut notes = Vec::new();
                    if !edits.is_empty() {
                        notes.push(String::from(
                            "The view reads kept fields by their plain names, without the `$` prefix (ADR 0059).",
                        ));
                    }
                    let relative = portable_path(path.strip_prefix(root).unwrap_or(&path));
                    let bound = field_rename::FieldRename::view_edits(&source, &bindings);
                    Self::add_view_bindings(&mut edits, &mut notes, bound);
                    Self::add_removed_controls(&mut edits, &mut notes, &source);
                    let braced = BracedAttributeEdits::plan_view(&source);
                    for (problem, action) in braced.steps {
                        self.manual_steps
                            .push(ManualStep::new(&relative, problem, action));
                    }
                    let fits = braced.edits.iter().all(|edit| Self::fits(&edits, edit));
                    if !braced.edits.is_empty() && fits {
                        edits.extend(braced.edits);
                        notes.push(String::from(
                            "A plain attribute with a value in braces becomes a `:` attribute, because braces work only in text (TY1313).",
                        ));
                    }
                    if edits.is_empty() {
                        continue;
                    }
                    self.changes.push(FileChange {
                        source: relative,
                        notes,
                        hunks: Hunk::from_edits(&source, &edits),
                        updated: TextEdit::apply(&source, &edits),
                        original: source,
                    });
                }
            }
        }
        Ok(())
    }

    fn add_python_parameters(
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        if std::path::Path::new(relative)
            .extension()
            .is_none_or(|value| value != "py")
        {
            return;
        }
        let Some(text) = scan::SourceText::read("py", source) else {
            return;
        };
        let mut changed = edits
            .iter()
            .any(|edit| edit.replacement.contains("Annotated["));
        let parameters = python_parameters::PythonParameterEdits::plan(&text)
            .into_iter()
            .map(|edit| ("parameter", edit));
        let fields = python_parameters::PythonParameterEdits::plan_fields(&text)
            .into_iter()
            .map(|edit| ("field", edit));
        for (word, (at, edit)) in parameters.chain(fields) {
            if Self::fits(edits, &edit) {
                let line = text.line_number(at);
                edits.push(edit);
                notes.push(format!(
                    "Moved the Python {word} marker on line {line} into the {word}'s type, as `name: Annotated[Type, marker]`."
                ));
                changed = true;
            }
        }
        let imported = edits
            .iter()
            .any(|edit| edit.replacement.contains("from typing import Annotated"));
        if changed
            && !imported
            && let Some(import) = python_parameters::PythonParameterEdits::import(source)
            && Self::fits(edits, &import)
        {
            edits.push(import);
        }
    }

    fn add_route(
        &mut self,
        relative: &str,
        source: &str,
        routes: &RouteMigration,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> Option<FileMove> {
        let file = routes.file(relative)?;
        match file.edits(source) {
            Ok(route) => {
                if !route.edits.iter().all(|edit| Self::fits(edits, edit)) {
                    self.manual_steps.push(Self::overlap(relative));
                    return None;
                }
                edits.extend(route.edits);
                notes.extend(route.notes);
                self.manual_steps.extend(route.steps);
                Some(FileMove {
                    from: file.from.clone(),
                    to: file.to.clone(),
                })
            }
            Err(step) => {
                self.manual_steps.push(step);
                None
            }
        }
    }

    fn overlap(relative: &str) -> ManualStep {
        ManualStep::new(
            relative,
            String::from("Two changes of the fix touch the same code, so the fix makes neither."),
            String::from("Run the fix again after you apply the other changes by hand."),
        )
    }

    fn add_renames(
        relative: &str,
        source: &str,
        roles: &RoleRename,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let (annotation_edits, annotation_notes) = AnnotationRename::plan(relative, source);
        let mut renamed_types = false;
        for (edit, note) in annotation_edits.into_iter().zip(annotation_notes) {
            if Self::fits(edits, &edit) {
                edits.push(edit);
                notes.push(note);
            }
        }
        for edit in roles.type_edits(source) {
            if Self::fits(edits, &edit) {
                edits.push(edit);
                renamed_types = true;
            }
        }
        if renamed_types {
            notes.push(String::from(
                "Renamed the role types that moved to their new names (ADR 0047).",
            ));
        }
        Self::add_path_renames(relative, source, roles, edits, notes);
    }

    fn add_path_renames(
        relative: &str,
        source: &str,
        roles: &RoleRename,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let before = edits.len();
        for edit in roles.path_edits(relative, source) {
            Self::add_path_edit(edits, edit);
        }
        notes.extend((edits.len() > before).then(|| {
            String::from("Changed the paths of files that moved to a renamed role folder.")
        }));
    }

    fn add_path_edit(edits: &mut Vec<TextEdit>, edit: TextEdit) {
        let accepted = Self::fits(edits, &edit).then_some(edit);
        edits.extend(accepted);
    }

    fn fits(edits: &[TextEdit], edit: &TextEdit) -> bool {
        edits
            .iter()
            .all(|other| edit.end <= other.start || other.end <= edit.start)
    }

    fn add_middleware(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> bool {
        if relative.contains('/') || !relative.starts_with("middleware.") {
            return false;
        }
        let mut plan = MiddlewareConversion::plan(relative, source);
        if let MiddlewarePlan::Converted {
            edits: conversion, ..
        } = &plan
            && !TextEdit::are_separate(&[edits.as_slice(), conversion.as_slice()].concat())
        {
            plan = MiddlewarePlan::manual("its changes would overlap the import changes");
        }
        match plan {
            MiddlewarePlan::Unchanged => false,
            MiddlewarePlan::Converted {
                edits: conversion,
                notes: conversion_notes,
            } => {
                edits.extend(conversion);
                notes.extend(conversion_notes);
                false
            }
            MiddlewarePlan::Manual { problem, action } => {
                self.manual_steps
                    .push(ManualStep::new(relative, problem, action));
                true
            }
        }
    }

    pub fn write(&self, project_root: impl AsRef<Path>) -> Result<(), Failure> {
        let root = project_root.as_ref();
        self.check_deletes(root)?;
        self.check_legacy_identity(root)?;
        self.check_moves(root)?;
        for change in &self.changes {
            if !MigrationFix::is_writable_target(&change.source) {
                return Err(migration_failure(&format!(
                    "'{}' is not a file that the fix may change.",
                    change.source
                )));
            }
            Self::check_path(root, &change.source)?;
            let path = root.join(&change.source);
            if !Self::is_unchanged(&path, change)? {
                return Err(migration_failure(&format!(
                    "'{}' changed after the fix was planned; run the command again.",
                    change.source
                )));
            }
        }
        for change in &self.changes {
            Self::write_change(root, change)?;
        }
        self.move_files(root)?;
        self.delete_files(root)?;
        self.delete_legacy_identity(root)?;
        if !self.moves.is_empty() || !self.deletes.is_empty() {
            RouteMigration::remove_empty_folders(root);
        }
        Ok(())
    }

    fn write_change(root: &Path, change: &FileChange) -> Result<(), Failure> {
        let path = root.join(&change.source);
        let cannot_write = |error: std::io::Error| {
            migration_failure(&format!("Cannot write '{}': {error}", change.source))
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(cannot_write)?;
        }
        fs::write(&path, &change.updated).map_err(cannot_write)
    }

    fn check_legacy_identity(&self, root: &Path) -> Result<(), Failure> {
        let Some(original) = &self.legacy_identity else {
            return Ok(());
        };
        let relative = ManifestMigration::LEGACY_IDENTITY;
        Self::check_path(root, relative)?;
        let path = root.join(relative);
        let metadata = MigrationFix::metadata(&path)?;
        let current = fs::read_to_string(&path).unwrap_or_default();
        if metadata.is_file() && !metadata.is_symlink() && current == *original {
            return Ok(());
        }
        Err(migration_failure(&format!(
            "'{relative}' changed after the fix was planned; run the command again."
        )))
    }

    fn delete_legacy_identity(&self, root: &Path) -> Result<(), Failure> {
        if self.legacy_identity.is_none() {
            return Ok(());
        }
        let relative = ManifestMigration::LEGACY_IDENTITY;
        fs::remove_file(root.join(relative))
            .map_err(|error| migration_failure(&format!("Cannot delete '{relative}': {error}")))
    }

    fn check_deletes(&self, root: &Path) -> Result<(), Failure> {
        for relative in &self.deletes {
            if !RouteMigration::is_deletable(relative) {
                return Err(migration_failure(&format!(
                    "'{relative}' is not a file that the fix may delete."
                )));
            }
            Self::check_path(root, relative)?;
            let path = root.join(relative);
            let metadata = MigrationFix::metadata(&path)?;
            let source = fs::read_to_string(&path).map_err(|error| {
                migration_failure(&format!("Cannot read '{relative}': {error}"))
            })?;
            let unchanged = metadata.is_file()
                && !metadata.is_symlink()
                && RouteMigration::is_blank(relative, &source);
            if !unchanged {
                return Err(migration_failure(&format!(
                    "'{relative}' changed after the fix was planned; run the command again."
                )));
            }
        }
        Ok(())
    }

    fn delete_files(&self, root: &Path) -> Result<(), Failure> {
        for relative in &self.deletes {
            fs::remove_file(root.join(relative)).map_err(|error| {
                migration_failure(&format!("Cannot delete '{relative}': {error}"))
            })?;
        }
        Ok(())
    }

    fn check_moves(&self, root: &Path) -> Result<(), Failure> {
        let mut destinations = std::collections::BTreeSet::new();
        for file_move in &self.moves {
            if !MigrationFix::is_allowed_move(file_move) {
                return Err(migration_failure(&format!(
                    "'{}' is not a file that the fix may move to '{}'.",
                    file_move.from, file_move.to
                )));
            }
            if !destinations.insert(file_move.to.to_lowercase()) {
                return Err(migration_failure(&format!(
                    "More than one file would move to '{}'; choose separate destination names.",
                    file_move.to,
                )));
            }
            Self::check_path(root, &file_move.from)?;
            Self::check_path(root, &file_move.to)?;
            let from = root.join(&file_move.from);
            let to = root.join(&file_move.to);
            let metadata = MigrationFix::metadata(&from)?;
            let taken = to.exists() && !file_move.changes_only_case();
            if !metadata.is_file() || metadata.is_symlink() || taken {
                return Err(migration_failure(&format!(
                    "'{}' cannot move to '{}'; run the command again.",
                    file_move.from, file_move.to
                )));
            }
        }
        Ok(())
    }

    fn check_path(root: &Path, relative: &str) -> Result<(), Failure> {
        let mut path = root.to_path_buf();
        for part in Path::new(relative).components() {
            path.push(part);
            match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_symlink() => {
                    return Err(migration_failure(&format!(
                        "'{relative}' contains a link; the fix does not follow links."
                    )));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => {
                    return Err(migration_failure(&format!(
                        "Cannot inspect '{relative}': {error}"
                    )));
                }
            }
        }
        Ok(())
    }

    fn move_files(&self, root: &Path) -> Result<(), Failure> {
        for file_move in &self.moves {
            let from = root.join(&file_move.from);
            let to = root.join(&file_move.to);
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    migration_failure(&format!(
                        "Cannot create the folder of '{}': {error}",
                        file_move.to
                    ))
                })?;
            }
            if file_move.changes_only_case() {
                fs::rename(&from, &to).map_err(|error| {
                    migration_failure(&format!("Cannot rename '{}': {error}", file_move.from))
                })?;
                continue;
            }
            fs::hard_link(&from, &to).map_err(|error| {
                migration_failure(&format!("Cannot move '{}': {error}", file_move.from))
            })?;
            fs::remove_file(&from).map_err(|error| {
                migration_failure(&format!(
                    "Copied '{}' to '{}', but could not remove the original: {error}",
                    file_move.from, file_move.to,
                ))
            })?;
        }
        Ok(())
    }

    fn is_unchanged(path: &Path, change: &FileChange) -> Result<bool, Failure> {
        if change.original.is_empty() && fs::symlink_metadata(path).is_err() {
            return Ok(true);
        }
        let metadata = MigrationFix::metadata(path)?;
        if !metadata.is_file() || metadata.is_symlink() {
            return Ok(false);
        }
        let current = fs::read_to_string(path).unwrap_or_default();
        Ok(current == change.original)
    }

    #[must_use]
    pub fn to_text(&self, written: bool) -> String {
        let mut text = String::new();
        let label = if written { "changed" } else { "change" };
        for change in &self.changes {
            let _ = writeln!(text, "{label:<8} {}", change.source);
            for note in &change.notes {
                let _ = writeln!(text, "         {note}");
            }
            for hunk in &change.hunks {
                Self::write_hunk(&mut text, hunk);
            }
        }
        let move_label = if written { "moved" } else { "move" };
        for file_move in &self.moves {
            let _ = writeln!(
                text,
                "{move_label:<8} {} -> {}",
                file_move.from, file_move.to
            );
        }
        let delete_label = if written { "deleted" } else { "delete" };
        for relative in &self.deletes {
            let _ = writeln!(text, "{delete_label:<8} {relative}");
            let _ = writeln!(
                text,
                "         This empty route file is no longer read (ADR 0048)."
            );
        }
        if self.legacy_identity.is_some() {
            let _ = writeln!(
                text,
                "{delete_label:<8} {}",
                ManifestMigration::LEGACY_IDENTITY
            );
            let _ = writeln!(
                text,
                "         Every value moved to client/manifest.json (ADR 0052)."
            );
        }
        for step in &self.manual_steps {
            let _ = writeln!(text, "{:<8} {}", "manual", step.source);
            let _ = writeln!(text, "         {}", step.problem);
            let _ = writeln!(text, "         action: {}", step.action);
        }
        text.push_str(&self.summary(written));
        text
    }

    fn write_hunk(text: &mut String, hunk: &Hunk) {
        let _ = writeln!(text, "         @@ line {} @@", hunk.line);
        for line in &hunk.removed {
            let _ = writeln!(text, "         - {line}");
        }
        for line in &hunk.added {
            let _ = writeln!(text, "         + {line}");
        }
    }

    fn summary(&self, written: bool) -> String {
        let changes = self.changes.len();
        let manual = self.manual_steps.len();
        let moves = self.move_summary(written);
        if written {
            return format!("\n{changes} file(s) changed, {moves}{manual} manual step(s)\n");
        }
        if changes == 0 && self.moves.is_empty() && self.delete_count() == 0 {
            return format!("\nNo automatic changes, {manual} manual step(s)\n");
        }
        format!(
            "\n{changes} file(s) to change, {moves}{manual} manual step(s). Run 'ty migrate fix --write' to apply the changes.\n"
        )
    }

    fn move_summary(&self, written: bool) -> String {
        let mut summary = String::new();
        let moves = self.moves.len();
        let deletes = self.delete_count();
        if moves != 0 {
            let action = if written { "moved" } else { "to move" };
            let _ = write!(summary, "{moves} file(s) {action}, ");
        }
        if deletes != 0 {
            let action = if written { "deleted" } else { "to delete" };
            let _ = write!(summary, "{deletes} file(s) {action}, ");
        }
        summary
    }
}
