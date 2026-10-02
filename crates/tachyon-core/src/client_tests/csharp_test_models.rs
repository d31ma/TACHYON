use crate::Failure;
use crate::route_types::{Converters, ModelTable};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) struct CsharpTestModels;

impl CsharpTestModels {
    pub(super) fn lower(
        root: &Path,
        path: &Path,
        source: &str,
        resources: &BTreeMap<String, BTreeSet<String>>,
    ) -> Result<Option<String>, Failure> {
        Self::selected_lower(root, path, source, resources)
    }

    pub(super) fn program(root: &Path, lowered: &str) -> Result<String, Failure> {
        Self::with_codec(root, lowered)
    }

    fn selected_lower(
        root: &Path,
        path: &Path,
        source: &str,
        resources: &BTreeMap<String, BTreeSet<String>>,
    ) -> Result<Option<String>, Failure> {
        if path.extension().is_some_and(|extension| extension == "cs") {
            let models = ModelTable::read_realm(root, "client", "cs")
                .map_err(|message| Self::failure(path, message))?;
            crate::behavior::CsharpTaskModels::lower(path, source, &models)
        } else {
            crate::behavior::lower_with_resources(path, source, "tac", resources)
        }
    }

    fn with_codec(root: &Path, lowered: &str) -> Result<String, Failure> {
        if lowered.contains("TacModels.ReadResult<") {
            let context = Converters::csharp_tac_test_text(root)
                .map_err(|message| Self::failure(Path::new("client/models"), message))?;
            Ok(format!(
                "{lowered}\n{}\n{context}",
                crate::behavior::CsharpTaskModels::codec()
            ))
        } else {
            Ok(lowered.to_owned())
        }
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
