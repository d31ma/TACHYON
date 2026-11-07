use crate::Failure;
use crate::route_types::ModelTable;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) struct CapturedBrowserBehavior;

impl CapturedBrowserBehavior {
    pub(super) fn lower(
        root: &Path,
        path: &Path,
        source: &str,
        resources: &BTreeMap<String, BTreeSet<String>>,
    ) -> Result<Option<String>, Failure> {
        Self::selected_lower(root, path, source, resources)
    }

    fn selected_lower(
        root: &Path,
        path: &Path,
        source: &str,
        resources: &BTreeMap<String, BTreeSet<String>>,
    ) -> Result<Option<String>, Failure> {
        if path.extension().is_some_and(|extension| extension == "cs") {
            Self::model_lower(root, path, source)
        } else {
            crate::behavior::lower_with_resources(path, source, "tac", resources)
        }
    }

    fn model_lower(root: &Path, path: &Path, source: &str) -> Result<Option<String>, Failure> {
        let models = ModelTable::read_realm(root, "client", "cs")
            .map_err(|message| Self::failure(path, message))?;
        crate::behavior::CsharpTaskModels::lower(path, source, &models)
    }

    fn failure(path: &Path, message: String) -> Failure {
        Failure::one(crate::failure::diagnostic(
            1310,
            message,
            None,
            crate::failure::source_span(&path.to_string_lossy(), 0, 1),
        ))
    }
}
