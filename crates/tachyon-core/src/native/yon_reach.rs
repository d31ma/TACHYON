use super::config::{WebAppManifest, config_failure, read_manifest};
use crate::{Failure, Project};
use std::path::Path;
use tachyon_contracts::NativeTarget;

pub(super) struct YonReach;

impl YonReach {
    const HIDDEN_PREFIX: char = '.';
    const MAX_FOLDER_DEPTH: usize = 32;

    pub(super) fn check(
        project: &Project,
        target: NativeTarget,
    ) -> Result<Option<String>, Failure> {
        let declared = read_manifest(project.snapshot_root())?
            .map(WebAppManifest::into_tac)
            .and_then(|tac| tac.yon)
            .is_some();
        let platform = super::compiler::native_target_directory(target);
        let remotes = Self::has_remotes(project.snapshot_root());
        Self::verdict([declared, remotes, Self::has_yon_routes(project)], platform)
    }

    fn verdict(found: [bool; 3], platform: &str) -> Result<Option<String>, Failure> {
        match found {
            [true, _, _] => Ok(None),
            [false, true, _] => Err(Self::refusal(platform)),
            [false, false, routes] => Ok(routes.then(|| Self::offline_note(platform))),
        }
    }

    fn refusal(platform: &str) -> Failure {
        config_failure(&format!(
            "The {platform} app calls Yon through client/remotes, but a native app has no Yon on its own origin. Add tac.yon.origin to client/manifest.json, such as {{\"tac\": {{\"yon\": {{\"origin\": \"https://api.example.com\"}}}}}}, and list the app origin in server.origins in server/application.json (ADR 0103)."
        ))
    }

    fn offline_note(platform: &str) -> String {
        format!(
            "Note: the {platform} app is offline-only. The project has Yon routes, but client/manifest.json declares no tac.yon.origin, so the app cannot reach them (ADR 0103)."
        )
    }

    fn has_yon_routes(project: &Project) -> bool {
        project
            .route_graph()
            .routes()
            .iter()
            .any(|route| !route.handlers().is_empty())
    }

    fn has_remotes(root: &Path) -> bool {
        Project::REMOTE_ROOTS
            .iter()
            .any(|folder| Self::holds_a_file(&root.join(folder), 0))
    }

    fn holds_a_file(folder: &Path, depth: usize) -> bool {
        std::fs::read_dir(folder)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| !Self::is_hidden(&entry.file_name()))
            .any(|entry| Self::is_or_holds_a_file(&entry, depth))
    }

    fn is_or_holds_a_file(entry: &std::fs::DirEntry, depth: usize) -> bool {
        let kind = entry.file_type().ok();
        let file = kind.is_some_and(|kind| kind.is_file());
        let folder = kind.is_some_and(|kind| kind.is_dir());
        let deeper = [folder, depth < Self::MAX_FOLDER_DEPTH] == [true, true];
        file || deeper && Self::holds_a_file(&entry.path(), depth + 1)
    }

    fn is_hidden(name: &std::ffi::OsStr) -> bool {
        name.to_string_lossy().starts_with(Self::HIDDEN_PREFIX)
    }
}
