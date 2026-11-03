use crate::Failure;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) struct ModelMetadata {
    files: Vec<(PathBuf, String)>,
}

impl ModelMetadata {
    pub(super) fn new(files: Vec<(PathBuf, String)>) -> Self {
        Self { files }
    }

    pub(crate) fn assembled(&self, path: &Path, source: &str) -> Result<String, Failure> {
        self.assemble_present(path, source)
    }

    fn assemble_present(&self, path: &Path, source: &str) -> Result<String, Failure> {
        if self.files.is_empty() {
            return Ok(source.to_owned());
        }
        crate::annotation_modules::Module::with_models(path, source, self.files.clone())
    }

    pub(crate) fn lowered(
        &self,
        entry: &Path,
        source: &str,
        lower: impl FnOnce(&str) -> Result<Option<String>, Failure>,
    ) -> Result<Option<String>, Failure> {
        let assembled = self.assembled(entry, source)?;
        lower(&assembled)?
            .map(|lowered| self.without_declarations(entry, &lowered))
            .transpose()
    }

    pub(crate) fn without_declarations(
        &self,
        path: &Path,
        source: &str,
    ) -> Result<String, Failure> {
        self.remove_present(path, source)
    }

    fn remove_present(&self, path: &Path, source: &str) -> Result<String, Failure> {
        if self.files.is_empty() {
            return Ok(source.to_owned());
        }
        self.without_types(path, source)
    }

    fn without_types(&self, path: &Path, source: &str) -> Result<String, Failure> {
        let names = self.names()?;
        let mut bytes = source.as_bytes().to_vec();
        crate::model_layout::ModelBlocks::schema_declarations(path, source)?
            .into_iter()
            .filter(|block| names.contains(&block.name))
            .for_each(|block| {
                crate::model_layout::JavaModelSchema::blank_bytes(
                    &mut bytes[block.start..block.end],
                );
            });
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn names(&self) -> Result<BTreeSet<String>, Failure> {
        self.files
            .iter()
            .try_fold(BTreeSet::new(), |mut names, (path, source)| {
                names.extend(crate::model_layout::ModelBlocks::names(path, source)?);
                Ok(names)
            })
    }
}
