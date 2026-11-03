use crate::Failure;
use crate::model_sources::ModelSources;
use std::fmt::Write as _;
use std::path::Path;

pub(super) struct DartTestModels;

impl DartTestModels {
    pub(super) fn program(
        project: &Path,
        models: &ModelSources,
        lowered: &str,
        portable: &str,
    ) -> Result<String, Failure> {
        let imports = Self::imports(models);
        let converters = crate::route_types::Converters::dart_tac_test_text(project)
            .map_err(|error| super::fail(portable, error))?;
        Ok(format!("{imports}{lowered}\n{converters}"))
    }

    fn imports(models: &ModelSources) -> String {
        models
            .separate_files()
            .iter()
            .fold(String::new(), |mut imports, (relative, _)| {
                let path = crate::compiler::portable_path(relative);
                let quoted = serde_json::Value::String(path)
                    .to_string()
                    .replace('$', "\\$");
                let _ = writeln!(imports, "import {quoted};");
                imports
            })
    }
}
