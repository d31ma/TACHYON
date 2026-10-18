use super::{
    MigrationAnalysis, is_ignored_directory, migration_failure, portable_path, sorted_entries,
};
use crate::Failure;
use crate::declared_contract::SourceEdit;
use braced_attributes::BracedAttributeEdits;
use clients::ClientConversion;
use conversion_plan::ConversionPlan;
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
use routes::{RouteEdits, RouteFile, RouteMigration};
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

mod arguments;
mod braced_attributes;
mod clients;
mod conversion_plan;
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
    const LANGUAGES: [&str; 13] = [
        "js", "ts", "py", "php", "java", "kt", "cs", "rs", "swift", "dart", "go", "rb", "cpp",
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
        let route_moves = plan.add_files(root, &sources, &roles, &routes);
        plan.add_unread_routes(&sources, &routes);
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
        let mut pending = Self::renamed_role_folders(root)?;
        while let Some(directory) = pending.pop() {
            Self::read_role_folder(root, &directory, &mut pending, &mut files, manual_steps)?;
        }
        files.sort();
        Ok(files)
    }

    fn renamed_role_folders(root: &Path) -> Result<Vec<PathBuf>, Failure> {
        let mut folders = Vec::new();
        for folder in Self::SOURCE_FOLDERS {
            folders.extend(Self::renamed_folders_in(root, folder)?);
        }
        Ok(folders)
    }

    fn renamed_folders_in(root: &Path, folder: &str) -> Result<Vec<PathBuf>, Failure> {
        let directory = root.join(folder);
        if !Self::is_real_directory(&directory) {
            return Ok(Vec::new());
        }
        let entries = sorted_entries(&directory)?;
        let renamed = entries
            .iter()
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                RoleRename::renamed_folder_files(&format!("{folder}/{name}/"))
            })
            .map(fs::DirEntry::path)
            .collect();
        Ok(renamed)
    }

    fn read_role_folder(
        root: &Path,
        directory: &Path,
        pending: &mut Vec<PathBuf>,
        files: &mut Vec<String>,
        manual_steps: &mut Vec<ManualStep>,
    ) -> Result<(), Failure> {
        if !Self::is_real_directory(directory) {
            let relative = portable_path(directory.strip_prefix(root).unwrap_or(directory));
            manual_steps.push(ManualStep::new(
                &relative,
                String::from(
                    "A renamed role folder is a link, so the fix does not follow it (ADR 0047).",
                ),
                String::from("Replace the link with a real folder under its new name."),
            ));
            return Ok(());
        }
        Self::read_role_entries(root, directory, pending, files, manual_steps)
    }

    fn read_role_entries(
        root: &Path,
        directory: &Path,
        pending: &mut Vec<PathBuf>,
        files: &mut Vec<String>,
        manual_steps: &mut Vec<ManualStep>,
    ) -> Result<(), Failure> {
        for entry in sorted_entries(directory)? {
            Self::read_role_entry(root, entry.path(), pending, files, manual_steps)?;
        }
        Ok(())
    }

    fn read_role_entry(
        root: &Path,
        path: PathBuf,
        pending: &mut Vec<PathBuf>,
        files: &mut Vec<String>,
        manual_steps: &mut Vec<ManualStep>,
    ) -> Result<(), Failure> {
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
        Ok(())
    }

    fn is_real_directory(path: &Path) -> bool {
        fs::symlink_metadata(path)
            .is_ok_and(|metadata| [metadata.is_dir(), metadata.is_symlink()] == [true, false])
    }

    fn project_files(root: &Path) -> Result<Vec<String>, Failure> {
        let mut files = Self::root_middleware_files(root)?;
        for folder in Self::SOURCE_FOLDERS {
            Self::collect_folder_sources(root, folder, &mut files)?;
        }
        files.sort();
        Ok(files)
    }

    fn root_middleware_files(root: &Path) -> Result<Vec<String>, Failure> {
        let mut files = Vec::new();
        for entry in sorted_entries(root)? {
            let name = entry.file_name().to_string_lossy().into_owned();
            let middleware = Self::is_middleware_source(&name, &entry.path())?;
            files.extend(middleware.then_some(name));
        }
        Ok(files)
    }

    fn is_middleware_source(name: &str, path: &Path) -> Result<bool, Failure> {
        let candidate = [name.starts_with("middleware."), Self::is_source(path)];
        if candidate != [true, true] {
            return Ok(false);
        }
        Self::is_small_file(path)
    }

    fn collect_folder_sources(
        root: &Path,
        folder: &str,
        files: &mut Vec<String>,
    ) -> Result<(), Failure> {
        let directory = root.join(folder);
        let is_directory = fs::symlink_metadata(&directory).is_ok_and(|metadata| metadata.is_dir());
        if is_directory {
            Self::collect_sources(root, &directory, files)?;
        }
        Ok(())
    }

    fn collect_sources(root: &Path, folder: &Path, files: &mut Vec<String>) -> Result<(), Failure> {
        let mut pending = vec![folder.to_path_buf()];
        while let Some(directory) = pending.pop() {
            Self::collect_directory(root, &directory, &mut pending, files)?;
        }
        Ok(())
    }

    fn collect_directory(
        root: &Path,
        directory: &Path,
        pending: &mut Vec<PathBuf>,
        files: &mut Vec<String>,
    ) -> Result<(), Failure> {
        for entry in sorted_entries(directory)? {
            Self::collect_entry(root, &entry, pending, files)?;
        }
        Ok(())
    }

    fn collect_entry(
        root: &Path,
        entry: &fs::DirEntry,
        pending: &mut Vec<PathBuf>,
        files: &mut Vec<String>,
    ) -> Result<(), Failure> {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let walks = Self::walks_into(&path, &name)?;
        pending.extend(walks.then(|| path.clone()));
        let small_source = Self::is_small_source(&path)?;
        let relative = path.strip_prefix(root).unwrap_or(&path);
        files.extend(small_source.then(|| portable_path(relative)));
        Ok(())
    }

    fn walks_into(path: &Path, name: &str) -> Result<bool, Failure> {
        let metadata = Self::metadata(path)?;
        let walked = [
            metadata.is_dir(),
            metadata.is_symlink(),
            is_ignored_directory(name),
        ];
        Ok(walked == [true, false, false])
    }

    fn is_small_source(path: &Path) -> Result<bool, Failure> {
        if !Self::is_source(path) {
            return Ok(false);
        }
        Self::is_small_file(path)
    }

    fn is_writable_target(relative: &str) -> bool {
        let is_manifest = relative == ManifestMigration::MANIFEST;
        let writable_source = [
            Self::is_normal(relative),
            Self::is_writable_source(relative),
        ];
        [is_manifest, writable_source == [true, true]].contains(&true)
    }

    fn is_writable_source(relative: &str) -> bool {
        let path = Path::new(relative);
        let is_root_middleware = [relative.contains('/'), relative.starts_with("middleware.")];
        if is_root_middleware == [false, true] {
            return Self::is_source(path);
        }
        let in_source_folder = Self::SOURCE_FOLDERS
            .iter()
            .any(|folder| relative.starts_with(&format!("{folder}/")));
        let is_client_view = [
            relative.starts_with("client/"),
            relative.ends_with("/tac.html"),
        ];
        let is_folder_source = [in_source_folder, Self::is_source(path)];
        [
            is_client_view == [true, true],
            is_folder_source == [true, true],
        ]
        .contains(&true)
    }

    fn is_allowed_move(file_move: &FileMove) -> bool {
        let (from, to) = (file_move.from.as_str(), file_move.to.as_str());
        let normal = [Self::is_normal(from), Self::is_normal(to)] == [true, true];
        let planned = [
            RoleRename::is_allowed_move(from, to),
            RouteMigration::is_allowed_move(from, to),
            FileNameFix::is_allowed_move(from, to),
        ];
        [normal, planned.contains(&true)] == [true, true]
    }

    fn is_normal(relative: &str) -> bool {
        Path::new(relative).components().all(Self::is_normal_part)
    }

    fn is_normal_part(part: Component<'_>) -> bool {
        matches!(part, Component::Normal(_))
    }

    fn is_source(path: &Path) -> bool {
        let extension = path.extension().and_then(|value| value.to_str());
        extension.is_some_and(|extension| Self::LANGUAGES.contains(&extension))
    }

    fn is_small_file(path: &Path) -> Result<bool, Failure> {
        let metadata = Self::metadata(path)?;
        let small = metadata.len() <= MigrationAnalysis::MAX_SOURCE_BYTES;
        Ok([metadata.is_file(), metadata.is_symlink(), small] == [true, false, true])
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

    fn add_files(
        &mut self,
        root: &Path,
        sources: &[String],
        roles: &RoleRename,
        routes: &RouteMigration,
    ) -> Vec<FileMove> {
        let mut route_moves = Vec::new();
        for relative in sources {
            route_moves.extend(self.add_file(root, relative, roles, routes));
        }
        route_moves
    }

    fn add_unread_routes(&mut self, sources: &[String], routes: &RouteMigration) {
        let unread = routes
            .files
            .iter()
            .filter(|file| !sources.contains(&file.from));
        let steps = unread.map(|file| {
            ManualStep::new(
                &file.from,
                String::from("This route file is too large for the fix to read (ADR 0048)."),
                format!("Move it to '{}' by hand and make it a controller.", file.to),
            )
        });
        self.manual_steps.extend(steps);
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
        let original = migration.original;
        let updated = TextEdit::apply(&original, &migration.edits);
        self.add_change(
            ManifestMigration::MANIFEST,
            original,
            updated,
            &migration.edits,
            migration.notes,
        );
    }

    fn add_change(
        &mut self,
        relative: &str,
        original: String,
        updated: String,
        edits: &[TextEdit],
        notes: Vec<String>,
    ) {
        if edits.is_empty() {
            return;
        }
        self.changes.push(FileChange {
            source: String::from(relative),
            notes,
            hunks: Hunk::from_edits(&original, edits),
            original,
            updated,
        });
    }

    fn add_file(
        &mut self,
        root: &Path,
        relative: &str,
        roles: &RoleRename,
        routes: &RouteMigration,
    ) -> Option<FileMove> {
        let source = fs::read_to_string(root.join(relative)).ok();
        self.add_unreadable_route(relative, routes, source.is_none());
        source.and_then(|source| self.add_source(root, relative, source, roles, routes))
    }

    fn add_unreadable_route(&mut self, relative: &str, routes: &RouteMigration, unreadable: bool) {
        let route = routes.file(relative).is_some();
        let step = ([unreadable, route] == [true, true]).then(|| {
            ManualStep::new(
                relative,
                String::from(
                    "This controller could not be read as UTF-8, so the fix leaves it unchanged.",
                ),
                String::from(
                    "Save the controller as readable UTF-8, then run 'ty migrate fix' again.",
                ),
            )
        });
        self.manual_steps.extend(step);
    }

    fn add_source(
        &mut self,
        root: &Path,
        relative: &str,
        source: String,
        roles: &RoleRename,
        routes: &RouteMigration,
    ) -> Option<FileMove> {
        if Self::extension(relative) == Some("rb") {
            self.add_ruby_file(root, relative, source);
            return None;
        }
        self.add_polyglot_file(root, relative, source, roles, routes)
    }

    fn add_polyglot_file(
        &mut self,
        root: &Path,
        relative: &str,
        source: String,
        roles: &RoleRename,
        routes: &RouteMigration,
    ) -> Option<FileMove> {
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
        let argument_plan = arguments::ArgumentConversion::plan(relative, &source);
        self.add_planned(relative, argument_plan, &mut edits, &mut notes);
        let event_plan = events::EventConversion::plan(root, relative, &source);
        self.add_planned(relative, event_plan, &mut edits, &mut notes);
        let stream_plan = streams::StreamConversion::plan(relative, &source);
        self.add_planned(relative, stream_plan, &mut edits, &mut notes);
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
        let retired = [
            route_move.is_none(),
            relative.starts_with(RouteMigration::ROOT),
        ];
        let kept = retired != [true, true];
        self.add_layout_steps(checked, &updated, middleware_is_manual, kept);
        self.add_change(relative, source, updated, &edits, notes);
        route_move
    }

    fn add_layout_steps(
        &mut self,
        checked: &str,
        updated: &str,
        middleware_is_manual: bool,
        kept: bool,
    ) {
        if !kept {
            return;
        }
        self.manual_steps
            .extend(HiddenAnnotation::steps(checked, updated));
        self.manual_steps.extend(OneClassReport::steps(
            checked,
            updated,
            middleware_is_manual,
        ));
    }

    fn add_ruby_file(&mut self, root: &Path, relative: &str, source: String) {
        let mut edits = Vec::new();
        let mut notes = Vec::new();
        let event_plan = events::EventConversion::plan(root, relative, &source);
        self.add_planned(relative, event_plan, &mut edits, &mut notes);
        let stream_plan = streams::StreamConversion::plan(relative, &source);
        self.add_planned(relative, stream_plan, &mut edits, &mut notes);
        Self::add_injected_fields(relative, &source, &mut edits, &mut notes);
        self.add_private_members(relative, &source, &mut edits, &mut notes);
        Self::add_simpler_forms(root, relative, &source, &mut edits, &mut notes);
        self.add_type_comments(relative, &mut edits, &mut notes);
        let updated = TextEdit::apply(&source, &edits);
        self.add_change(relative, source, updated, &edits, notes);
    }

    fn extension(relative: &str) -> Option<&str> {
        Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
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
                self.add_or_report_overlap(relative, conversion, conversion_notes, edits, notes);
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
        let rewritten =
            crate::declared_contract::DeclaredContract::rewrite(Path::new(relative), source);
        match rewritten {
            Ok(planned) => self.add_contract_edits(relative, &planned, edits, notes),
            Err(problem) => self.manual_steps.push(ManualStep::new(
                relative,
                problem,
                String::from("Declare the HTTPClient or Store as a contract without bodies (TY1308), then run 'ty migrate fix --write' again."),
            )),
        }
    }

    fn add_contract_edits(
        &mut self,
        relative: &str,
        planned: &[SourceEdit],
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let contract: Vec<TextEdit> = planned
            .iter()
            .filter(|(range, text)| [range.is_empty(), text.is_empty()] != [true, true])
            .map(|(range, text)| TextEdit::replace(range.start, range.end, text))
            .collect();
        let note = (!contract.is_empty()).then(|| String::from(
            "Declared the HTTPClient or Store as a contract without an implementation: Tachyon generates each request and query (TY1308). Inject a Store with Needs instead of building it.",
        ));
        let contract_notes = note.into_iter().collect();
        self.add_or_report_overlap(relative, contract, contract_notes, edits, notes);
    }

    fn add_planned(
        &mut self,
        relative: &str,
        planned: ConversionPlan,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        self.manual_steps.extend(planned.steps);
        if planned.edits.is_empty() {
            return;
        }
        self.add_or_report_overlap(relative, planned.edits, planned.notes, edits, notes);
    }

    fn add_or_report_overlap(
        &mut self,
        relative: &str,
        planned: Vec<TextEdit>,
        planned_notes: Vec<String>,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let fits = Self::add_together(edits, notes, planned, planned_notes);
        self.manual_steps
            .extend((!fits).then(|| Self::overlap(relative)));
    }

    fn add_together(
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
        planned: Vec<TextEdit>,
        planned_notes: Vec<String>,
    ) -> bool {
        let fits = Self::all_fit(edits, &planned);
        if fits {
            edits.extend(planned);
            notes.extend(planned_notes);
        }
        fits
    }

    fn add_fitting(
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
        planned: impl IntoIterator<Item = (TextEdit, String)>,
    ) -> bool {
        let mut every_edit_fits = true;
        for (edit, note) in planned {
            let fits = Self::fits(edits, &edit);
            edits.extend(fits.then_some(edit));
            notes.extend(fits.then_some(note));
            every_edit_fits &= fits;
        }
        every_edit_fits
    }

    fn add_each_fitting(edits: &mut Vec<TextEdit>, planned: Vec<TextEdit>) -> bool {
        let before = edits.len();
        for edit in planned {
            let accepted = Self::fits(edits, &edit).then_some(edit);
            edits.extend(accepted);
        }
        edits.len() > before
    }

    fn add_go_markers(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let is_go = Self::extension(relative) == Some("go");
        let text = is_go
            .then(|| scan::SourceText::read("go", source))
            .flatten();
        if let Some(text) = text {
            self.add_go_marker_text(relative, &text, edits, notes);
        }
    }

    fn add_go_marker_text(
        &mut self,
        relative: &str,
        text: &scan::SourceText<'_>,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        for (start, _) in text.source.match_indices("#[") {
            self.add_go_marker(relative, text, start, edits, notes);
        }
    }

    fn add_go_marker(
        &mut self,
        relative: &str,
        text: &scan::SourceText<'_>,
        start: usize,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let Some(end) = Self::go_marker_end(text, start) else {
            return;
        };
        let line = text.line_number(start);
        let planned = go_markers::GoMarkerEdits::plan(text, start, end);
        let manual = planned.is_none().then(|| {
            ManualStep::new(
                relative,
                format!("The Go annotation on line {line} has a comment or a multi-line text value inside it, so it cannot become one `// @Name(...)` line."),
                String::from("Write it as `// @Name(...)` on one line (ADR 0050), then run 'ty migrate fix' again."),
            )
        });
        self.manual_steps.extend(manual);
        let fitting = planned.filter(|planned| Self::all_fit(edits, planned));
        let note = format!("Changed Go annotation to // @ on line {line} (ADR 0050).");
        notes.extend(fitting.is_some().then_some(note));
        edits.extend(fitting.into_iter().flatten());
    }

    fn go_marker_end(text: &scan::SourceText<'_>, start: usize) -> Option<usize> {
        text.is_code(start).then_some(())?;
        text.matching(start + 1)
    }

    fn add_native_http(
        &mut self,
        relative: &str,
        source: &str,
        middleware_converted: bool,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let extension = Self::extension(relative);
        let read = extension.and_then(|extension| scan::SourceText::read(extension, source));
        let text = read.zip(extension);
        let manual = text.and_then(|(text, extension)| {
            Self::add_native_http_text(extension, &text, middleware_converted, edits, notes)
        });
        let step = manual.map(|(problem, action)| ManualStep::new(relative, problem, action));
        self.manual_steps.extend(step);
    }

    fn add_native_http_text(
        extension: &str,
        text: &scan::SourceText<'_>,
        middleware_converted: bool,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> Option<(String, String)> {
        let plan = native_http::NativeHttpFix::plan(extension, text, middleware_converted);
        let planned = plan.edits.into_iter().zip(plan.notes);
        let every_edit_fits = Self::add_fitting(edits, notes, planned);
        let overlap =
            (!every_edit_fits).then(|| native_http::NativeHttpFix::manual_step_for(extension));
        plan.manual.or(overlap)
    }

    fn add_needs_all(
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let extension = Self::extension(relative);
        let supported = extension.filter(|extension| {
            crate::annotation_rules::AnnotationRules::LANGUAGES.contains(extension)
        });
        let text = supported.and_then(|extension| scan::SourceText::read(extension, source));
        let planned = text
            .map(|text| Self::needs_all_edits(&text))
            .unwrap_or_default();
        Self::add_fitting(edits, notes, planned);
    }

    fn needs_all_edits(text: &scan::SourceText<'_>) -> Vec<(TextEdit, String)> {
        needs_all::NeedsAllEdits::plan(text)
            .into_iter()
            .map(|(at, edit)| {
                let line = text.line_number(at);
                let note = format!(
                    "Removed the NeedsAll field list on line {line}: NeedsAll fills every field (ADR 0056)."
                );
                (edit, note)
            })
            .collect()
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
        if relative.starts_with("client/") {
            self.add_client_stored_fields(relative, source, edits, notes);
        }
    }

    fn add_client_stored_fields(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let planned = stored_fields::StoredFieldEdits::plan(relative, source);
        let clashes = planned
            .clashes
            .iter()
            .map(|written| Self::stored_field_clash(relative, written));
        self.manual_steps.extend(clashes);
        Self::add_together(edits, notes, planned.edits, planned.notes);
    }

    fn stored_field_clash(relative: &str, written: &str) -> ManualStep {
        let bare = written.trim_start_matches('$');
        ManualStep::new(
            relative,
            format!(
                "The field `{written}` keeps its value through a `$` prefix, and the class already uses the name `{bare}` (ADR 0059)."
            ),
            format!(
                "Rename one of them, then write @{} on the field.",
                crate::behavior::StoredState::marker_for(written)
            ),
        )
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
            Self::add_together(edits, notes, member_edits, vec![note]);
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
        Self::add_fitting(edits, notes, planned);
        let steps = type_comments_fix::TypeCommentsFix::steps(relative, &self.missing_types);
        let manual = steps
            .into_iter()
            .map(|(problem, action)| ManualStep::new(relative, problem, action));
        self.manual_steps.extend(manual);
    }

    fn add_view_edits(
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
        planned: Vec<TextEdit>,
        note: &str,
    ) {
        let fits = Self::all_fit(edits, &planned);
        if [fits, planned.is_empty()] == [true, false] {
            edits.extend(planned);
            notes.push(String::from(note));
        }
    }

    fn all_fit(edits: &[TextEdit], planned: &[TextEdit]) -> bool {
        planned.iter().all(|edit| Self::fits(edits, edit))
    }

    fn add_stored_views(&mut self, root: &Path) -> Result<(), Failure> {
        let bindings = field_rename::FieldRename::view_bindings(root);
        let mut pending = vec![root.join("client")];
        while let Some(directory) = pending.pop() {
            self.add_view_folder(root, &directory, &bindings, &mut pending)?;
        }
        Ok(())
    }

    fn add_view_folder(
        &mut self,
        root: &Path,
        directory: &Path,
        bindings: &[(String, String)],
        pending: &mut Vec<PathBuf>,
    ) -> Result<(), Failure> {
        if !MigrationFix::is_real_directory(directory) {
            return Ok(());
        }
        self.add_view_entries(root, directory, bindings, pending)
    }

    fn add_view_entries(
        &mut self,
        root: &Path,
        directory: &Path,
        bindings: &[(String, String)],
        pending: &mut Vec<PathBuf>,
    ) -> Result<(), Failure> {
        for entry in sorted_entries(directory)? {
            self.add_view_entry(root, &entry, bindings, pending)?;
        }
        Ok(())
    }

    fn add_view_entry(
        &mut self,
        root: &Path,
        entry: &fs::DirEntry,
        bindings: &[(String, String)],
        pending: &mut Vec<PathBuf>,
    ) -> Result<(), Failure> {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let walks = MigrationFix::walks_into(&path, &name)?;
        pending.extend(walks.then(|| path.clone()));
        let is_view = Self::is_view_file(&name, &path)?;
        let source = is_view.then(|| fs::read_to_string(&path).ok()).flatten();
        self.add_view_source(root, &path, source, bindings);
        Ok(())
    }

    fn add_view_source(
        &mut self,
        root: &Path,
        path: &Path,
        source: Option<String>,
        bindings: &[(String, String)],
    ) {
        if let Some(source) = source {
            self.add_view(root, path, source, bindings);
        }
    }

    fn is_view_file(name: &str, path: &Path) -> Result<bool, Failure> {
        if name != "tac.html" {
            return Ok(false);
        }
        MigrationFix::is_small_file(path)
    }

    fn add_view(
        &mut self,
        root: &Path,
        path: &Path,
        source: String,
        bindings: &[(String, String)],
    ) {
        let relative = portable_path(path.strip_prefix(root).unwrap_or(path));
        let mut edits = stored_fields::StoredFieldEdits::plan_view(&source);
        let kept_fields = (!edits.is_empty()).then(|| {
            String::from(
                "The view reads kept fields by their plain names, without the `$` prefix (ADR 0059).",
            )
        });
        let mut notes: Vec<String> = kept_fields.into_iter().collect();
        let bound = field_rename::FieldRename::view_edits(&source, bindings);
        Self::add_view_edits(
            &mut edits,
            &mut notes,
            bound,
            "The view reads each renamed injected field by its new name (TY1332).",
        );
        let controls = RemovedControlEdits::plan_view(&source).edits;
        Self::add_view_edits(
            &mut edits,
            &mut notes,
            controls,
            "The removed <if>, <else> and <for> elements become <logic :if>, <logic else> and <loop :for> (TY1367).",
        );
        let braced = BracedAttributeEdits::plan_view(&source);
        let braced_steps = braced
            .steps
            .into_iter()
            .map(|(problem, action)| ManualStep::new(&relative, problem, action));
        self.manual_steps.extend(braced_steps);
        Self::add_view_edits(
            &mut edits,
            &mut notes,
            braced.edits,
            "A plain attribute with a value in braces becomes a `:` attribute, because braces work only in text (TY1313).",
        );
        let updated = TextEdit::apply(&source, &edits);
        self.add_change(&relative, source, updated, &edits, notes);
    }

    fn add_python_parameters(
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let is_python = Self::extension(relative) == Some("py");
        let text = is_python
            .then(|| scan::SourceText::read("py", source))
            .flatten();
        if let Some(text) = text {
            Self::add_python_markers(source, &text, edits, notes);
        }
    }

    fn add_python_markers(
        source: &str,
        text: &scan::SourceText<'_>,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) {
        let annotated = edits
            .iter()
            .any(|edit| edit.replacement.contains("Annotated["));
        let parameters = python_parameters::PythonParameterEdits::plan(text)
            .into_iter()
            .map(|(at, edit)| (edit, Self::python_marker_note("parameter", text, at)));
        let fields = python_parameters::PythonParameterEdits::plan_fields(text)
            .into_iter()
            .map(|(at, edit)| (edit, Self::python_marker_note("field", text, at)));
        let before = edits.len();
        Self::add_fitting(edits, notes, parameters.chain(fields));
        let changed = [annotated, edits.len() > before].contains(&true);
        let imported = edits
            .iter()
            .any(|edit| edit.replacement.contains("from typing import Annotated"));
        let needs_import = [changed, imported] == [true, false];
        let import = needs_import
            .then(|| python_parameters::PythonParameterEdits::import(source))
            .flatten();
        let fitting_import = import.filter(|import| Self::fits(edits, import));
        edits.extend(fitting_import);
    }

    fn python_marker_note(word: &str, text: &scan::SourceText<'_>, at: usize) -> String {
        let line = text.line_number(at);
        format!(
            "Moved the Python {word} marker on line {line} into the {word}'s type, as `name: Annotated[Type, marker]`."
        )
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
            Ok(route) => self.add_route_edits(relative, file, route, edits, notes),
            Err(step) => {
                self.manual_steps.push(step);
                None
            }
        }
    }

    fn add_route_edits(
        &mut self,
        relative: &str,
        file: &RouteFile,
        route: RouteEdits,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> Option<FileMove> {
        let fits = Self::add_together(edits, notes, route.edits, route.notes);
        self.manual_steps
            .extend((!fits).then(|| Self::overlap(relative)));
        let accepted_steps = fits.then_some(route.steps);
        self.manual_steps
            .extend(accepted_steps.into_iter().flatten());
        fits.then(|| FileMove {
            from: file.from.clone(),
            to: file.to.clone(),
        })
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
        Self::add_fitting(
            edits,
            notes,
            annotation_edits.into_iter().zip(annotation_notes),
        );
        let renamed_types = Self::add_each_fitting(edits, roles.type_edits(source));
        notes.extend(renamed_types.then(|| {
            String::from("Renamed the role types that moved to their new names (ADR 0047).")
        }));
        let moved_paths = Self::add_each_fitting(edits, roles.path_edits(relative, source));
        notes.extend(moved_paths.then(|| {
            String::from("Changed the paths of files that moved to a renamed role folder.")
        }));
    }

    fn fits(edits: &[TextEdit], edit: &TextEdit) -> bool {
        edits.iter().all(|other| Self::are_apart(edit, other))
    }

    fn are_apart(edit: &TextEdit, other: &TextEdit) -> bool {
        [edit.end <= other.start, other.end <= edit.start].contains(&true)
    }

    fn add_middleware(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> bool {
        let root_middleware = [relative.contains('/'), relative.starts_with("middleware.")];
        if root_middleware != [false, true] {
            return false;
        }
        self.add_root_middleware(relative, source, edits, notes)
    }

    fn add_root_middleware(
        &mut self,
        relative: &str,
        source: &str,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> bool {
        let planned = MiddlewareConversion::plan(relative, source);
        let plan = Self::apart_from_imports(planned, edits);
        self.add_middleware_plan(relative, plan, edits, notes)
    }

    fn apart_from_imports(plan: MiddlewarePlan, edits: &[TextEdit]) -> MiddlewarePlan {
        if let MiddlewarePlan::Converted {
            edits: conversion, ..
        } = &plan
            && !TextEdit::are_separate(&[edits, conversion.as_slice()].concat())
        {
            return MiddlewarePlan::manual("its changes would overlap the import changes");
        }
        plan
    }

    fn add_middleware_plan(
        &mut self,
        relative: &str,
        plan: MiddlewarePlan,
        edits: &mut Vec<TextEdit>,
        notes: &mut Vec<String>,
    ) -> bool {
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
        self.check_changes(root)?;
        self.write_changes(root)?;
        self.move_files(root)?;
        self.delete_files(root)?;
        self.delete_legacy_identity(root)?;
        self.remove_empty_folders(root);
        Ok(())
    }

    fn ensure(condition: bool, message: &str) -> Result<(), Failure> {
        condition
            .then_some(())
            .ok_or_else(|| migration_failure(message))
    }

    fn check_changes(&self, root: &Path) -> Result<(), Failure> {
        for change in &self.changes {
            Self::check_change(root, change)?;
        }
        Ok(())
    }

    fn check_change(root: &Path, change: &FileChange) -> Result<(), Failure> {
        Self::ensure(
            MigrationFix::is_writable_target(&change.source),
            &format!("'{}' is not a file that the fix may change.", change.source),
        )?;
        Self::check_path(root, &change.source)?;
        let path = root.join(&change.source);
        Self::ensure(
            Self::is_unchanged(&path, change)?,
            &format!(
                "'{}' changed after the fix was planned; run the command again.",
                change.source
            ),
        )
    }

    fn write_changes(&self, root: &Path) -> Result<(), Failure> {
        for change in &self.changes {
            Self::write_change(root, change)?;
        }
        Ok(())
    }

    fn remove_empty_folders(&self, root: &Path) {
        if [self.moves.is_empty(), self.deletes.is_empty()] != [true, true] {
            RouteMigration::remove_empty_folders(root);
        }
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
        self.legacy_identity
            .as_ref()
            .map_or(Ok(()), |original| Self::check_legacy_file(root, original))
    }

    fn check_legacy_file(root: &Path, original: &str) -> Result<(), Failure> {
        let relative = ManifestMigration::LEGACY_IDENTITY;
        Self::check_path(root, relative)?;
        let path = root.join(relative);
        let metadata = MigrationFix::metadata(&path)?;
        let current = fs::read_to_string(&path).unwrap_or_default();
        let checks = [
            metadata.is_file(),
            metadata.is_symlink(),
            current == original,
        ];
        Self::ensure(
            checks == [true, false, true],
            &format!("'{relative}' changed after the fix was planned; run the command again."),
        )
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
            Self::check_delete(root, relative)?;
        }
        Ok(())
    }

    fn check_delete(root: &Path, relative: &str) -> Result<(), Failure> {
        Self::ensure(
            RouteMigration::is_deletable(relative),
            &format!("'{relative}' is not a file that the fix may delete."),
        )?;
        Self::check_path(root, relative)?;
        let path = root.join(relative);
        let metadata = MigrationFix::metadata(&path)?;
        let source = fs::read_to_string(&path)
            .map_err(|error| migration_failure(&format!("Cannot read '{relative}': {error}")))?;
        let checks = [
            metadata.is_file(),
            metadata.is_symlink(),
            RouteMigration::is_blank(relative, &source),
        ];
        Self::ensure(
            checks == [true, false, true],
            &format!("'{relative}' changed after the fix was planned; run the command again."),
        )
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
            Self::check_move(root, file_move, &mut destinations)?;
        }
        Ok(())
    }

    fn check_move(
        root: &Path,
        file_move: &FileMove,
        destinations: &mut std::collections::BTreeSet<String>,
    ) -> Result<(), Failure> {
        Self::ensure(
            MigrationFix::is_allowed_move(file_move),
            &format!(
                "'{}' is not a file that the fix may move to '{}'.",
                file_move.from, file_move.to
            ),
        )?;
        Self::ensure(
            destinations.insert(file_move.to.to_lowercase()),
            &format!(
                "More than one file would move to '{}'; choose separate destination names.",
                file_move.to,
            ),
        )?;
        Self::check_path(root, &file_move.from)?;
        Self::check_path(root, &file_move.to)?;
        let from = root.join(&file_move.from);
        let to = root.join(&file_move.to);
        let metadata = MigrationFix::metadata(&from)?;
        let taken = [to.exists(), file_move.changes_only_case()] == [true, false];
        Self::ensure(
            [metadata.is_file(), metadata.is_symlink(), taken] == [true, false, false],
            &format!(
                "'{}' cannot move to '{}'; run the command again.",
                file_move.from, file_move.to
            ),
        )
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
            Self::move_file(root, file_move)?;
        }
        Ok(())
    }

    fn move_file(root: &Path, file_move: &FileMove) -> Result<(), Failure> {
        let from = root.join(&file_move.from);
        let to = root.join(&file_move.to);
        let parent = to.parent();
        parent.map_or(Ok(()), |parent| {
            fs::create_dir_all(parent).map_err(|error| {
                migration_failure(&format!(
                    "Cannot create the folder of '{}': {error}",
                    file_move.to
                ))
            })
        })?;
        if file_move.changes_only_case() {
            return fs::rename(&from, &to).map_err(|error| {
                migration_failure(&format!("Cannot rename '{}': {error}", file_move.from))
            });
        }
        fs::hard_link(&from, &to).map_err(|error| {
            migration_failure(&format!("Cannot move '{}': {error}", file_move.from))
        })?;
        fs::remove_file(&from).map_err(|error| {
            migration_failure(&format!(
                "Copied '{}' to '{}', but could not remove the original: {error}",
                file_move.from, file_move.to,
            ))
        })
    }

    fn is_unchanged(path: &Path, change: &FileChange) -> Result<bool, Failure> {
        let missing = fs::symlink_metadata(path).is_err();
        if [change.original.is_empty(), missing] == [true, true] {
            return Ok(true);
        }
        let metadata = MigrationFix::metadata(path)?;
        let regular = [metadata.is_file(), metadata.is_symlink()] == [true, false];
        let current = regular.then(|| fs::read_to_string(path).unwrap_or_default());
        Ok(current.is_some_and(|current| current == change.original))
    }

    #[must_use]
    pub fn to_text(&self, written: bool) -> String {
        let mut text = String::new();
        let delete_label = Self::label(written, "deleted", "delete");
        self.write_change_lines(&mut text, Self::label(written, "changed", "change"));
        self.write_move_lines(&mut text, Self::label(written, "moved", "move"));
        self.write_delete_lines(&mut text, delete_label);
        self.write_legacy_identity_lines(&mut text, delete_label);
        self.write_manual_step_lines(&mut text);
        text.push_str(&self.summary(written));
        text
    }

    fn label(written: bool, done: &'static str, planned: &'static str) -> &'static str {
        if written {
            return done;
        }
        planned
    }

    fn write_change_lines(&self, text: &mut String, label: &str) {
        for change in &self.changes {
            Self::write_change_text(text, label, change);
        }
    }

    fn write_change_text(text: &mut String, label: &str, change: &FileChange) {
        let _ = writeln!(text, "{label:<8} {}", change.source);
        Self::write_lines(text, "", &change.notes);
        for hunk in &change.hunks {
            Self::write_hunk(text, hunk);
        }
    }

    fn write_lines(text: &mut String, marker: &str, lines: &[String]) {
        for line in lines {
            let _ = writeln!(text, "         {marker}{line}");
        }
    }

    fn write_move_lines(&self, text: &mut String, label: &str) {
        for file_move in &self.moves {
            let _ = writeln!(text, "{label:<8} {} -> {}", file_move.from, file_move.to);
        }
    }

    fn write_delete_lines(&self, text: &mut String, label: &str) {
        for relative in &self.deletes {
            let _ = writeln!(text, "{label:<8} {relative}");
            let _ = writeln!(
                text,
                "         This empty route file is no longer read (ADR 0048)."
            );
        }
    }

    fn write_legacy_identity_lines(&self, text: &mut String, label: &str) {
        if self.legacy_identity.is_none() {
            return;
        }
        let _ = writeln!(text, "{label:<8} {}", ManifestMigration::LEGACY_IDENTITY);
        let _ = writeln!(
            text,
            "         Every value moved to client/manifest.json (ADR 0052)."
        );
    }

    fn write_manual_step_lines(&self, text: &mut String) {
        for step in &self.manual_steps {
            let _ = writeln!(text, "{:<8} {}", "manual", step.source);
            let _ = writeln!(text, "         {}", step.problem);
            let _ = writeln!(text, "         action: {}", step.action);
        }
    }

    fn write_hunk(text: &mut String, hunk: &Hunk) {
        let _ = writeln!(text, "         @@ line {} @@", hunk.line);
        Self::write_lines(text, "- ", &hunk.removed);
        Self::write_lines(text, "+ ", &hunk.added);
    }

    fn summary(&self, written: bool) -> String {
        let changes = self.changes.len();
        let manual = self.manual_steps.len();
        let moves = self.move_summary(written);
        let nothing_planned = [changes, self.moves.len(), self.delete_count()] == [0, 0, 0];
        match (written, nothing_planned) {
            (true, _) => format!("\n{changes} file(s) changed, {moves}{manual} manual step(s)\n"),
            (false, true) => format!("\nNo automatic changes, {manual} manual step(s)\n"),
            (false, false) => format!(
                "\n{changes} file(s) to change, {moves}{manual} manual step(s). Run 'ty migrate fix --write' to apply the changes.\n"
            ),
        }
    }

    fn move_summary(&self, written: bool) -> String {
        let move_count = self.moves.len();
        let delete_count = self.delete_count();
        let moved = (move_count != 0).then(|| {
            let action = Self::label(written, "moved", "to move");
            format!("{move_count} file(s) {action}, ")
        });
        let deleted = (delete_count != 0).then(|| {
            let action = Self::label(written, "deleted", "to delete");
            format!("{delete_count} file(s) {action}, ")
        });
        format!(
            "{}{}",
            moved.unwrap_or_default(),
            deleted.unwrap_or_default()
        )
    }
}
