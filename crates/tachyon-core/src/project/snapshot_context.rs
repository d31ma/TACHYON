use super::CapturedFile;
use crate::Failure;
use crate::handler::{HandlerSource, OwnedSourceRoot};
use cap_std::fs::Dir;
use std::path::Path;
use std::sync::Arc;
use tachyon_diagnostics::Diagnostic;

pub(super) struct SnapshotContext<'a> {
    pub(super) project_root: &'a Path,
    pub(super) project: &'a Dir,
    pub(super) source_root: &'a Arc<OwnedSourceRoot>,
}

impl SnapshotContext<'_> {
    pub(super) fn discover_handler(
        &self,
        relative: &Path,
        bytes: Vec<u8>,
    ) -> Result<HandlerSource, Failure> {
        HandlerSource::discover_snapshot(
            self.project_root.to_path_buf(),
            self.project,
            relative,
            bytes,
            Arc::clone(self.source_root),
        )
    }

    pub(super) fn middleware(
        &self,
        file: Option<CapturedFile>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<HandlerSource> {
        let file = file?;
        self.discover_handler(&file.relative, file.bytes)
            .map_err(|failure| diagnostics.extend_from_slice(failure.diagnostics()))
            .ok()
    }
}
