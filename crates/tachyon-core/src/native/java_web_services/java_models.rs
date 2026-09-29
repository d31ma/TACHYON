use super::java_model::JavaModel;
use super::java_model_reader::JavaModelReader;
use super::java_model_table::JavaModelTable;
use super::java_value_type::JavaValueType;
use crate::Failure;
use std::path::{Path, PathBuf};

pub(super) struct JavaModels {
    files: Vec<(PathBuf, Vec<JavaModel>, String)>,
}

impl JavaModels {
    const REALM: &'static str = "client";
    const EXTENSION: &'static str = "java";

    pub(super) fn read(snapshot_root: &Path) -> Result<Self, Failure> {
        let sources =
            crate::model_sources::ModelSources::read(snapshot_root, Self::REALM, Self::EXTENSION)?;
        let files = sources
            .project_files()
            .into_iter()
            .map(|(path, source)| {
                let compiled = crate::topics::Topics::blanked(&source);
                let models = JavaModelReader::models(&compiled);
                (path, models, compiled)
            })
            .collect();
        Ok(Self { files })
    }

    pub(super) fn values(&self) -> JavaValueType {
        let names = self.models().into_iter().map(|model| model.name).collect();
        JavaValueType::new(names)
    }

    pub(super) fn sources(
        &self,
        values: &JavaValueType,
    ) -> Result<Vec<(PathBuf, String)>, Failure> {
        let models = self.models();
        let table = JavaModelTable::new(&models, values);
        self.files
            .iter()
            .try_for_each(|(path, declared, _)| Self::check(&table, path, declared))?;
        let (name, text) = table.file();
        let mut sources: Vec<(PathBuf, String)> = self
            .files
            .iter()
            .map(|(path, _, compiled)| (path.clone(), compiled.clone()))
            .collect();
        sources.push((PathBuf::from(name), text));
        Ok(sources)
    }

    fn models(&self) -> Vec<JavaModel> {
        self.files
            .iter()
            .flat_map(|(_, models, _)| models.iter().cloned())
            .collect()
    }

    fn check(
        table: &JavaModelTable<'_>,
        path: &Path,
        declared: &[JavaModel],
    ) -> Result<(), Failure> {
        let refusal = declared.iter().find_map(|model| table.refusal(model));
        refusal.map_or(Ok(()), |message| {
            Err(super::super::host::native_tool_failure(
                1405,
                &format!("{}: {message}", path.display()),
            ))
        })
    }
}
