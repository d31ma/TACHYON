use super::converter_models::ConverterModels;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::route_types::{MappingModels, ModelTable};
use std::path::Path;

pub(crate) struct ConverterModelTable<'a> {
    root: &'a Path,
    realm: &'static str,
    extension: &'a str,
    table: Option<ModelTable>,
}

impl<'a> ConverterModelTable<'a> {
    const CODE: u16 = 1308;
    const FOLDER: &'static str = "converters";
    const TAC: &'static str = "tac";
    const CLIENT: &'static str = "client";
    const SERVER: &'static str = "server";

    pub(crate) fn new(root: &'a Path, environment: &str, extension: &'a str) -> Self {
        let realm = Self::realm_of(environment);
        Self {
            root,
            realm,
            extension,
            table: None,
        }
    }

    fn realm_of(environment: &str) -> &'static str {
        if environment == Self::TAC {
            Self::CLIENT
        } else {
            Self::SERVER
        }
    }

    fn environment_of(&self) -> &'static str {
        if self.realm == Self::CLIENT {
            "tac"
        } else {
            "yon"
        }
    }

    pub(crate) fn written(&mut self, path: &Path, text: String) -> Result<String, Failure> {
        let in_converters = path
            .iter()
            .nth(1)
            .is_some_and(|folder| folder == Self::FOLDER);
        let written = in_converters
            .then(|| self.with_models(path, &text))
            .transpose()?;
        Ok(written.flatten().unwrap_or(text))
    }

    fn with_models(&mut self, path: &Path, text: &str) -> Result<Option<String>, Failure> {
        self.load_models(path)?;
        let models = self
            .table
            .as_ref()
            .map(|table| MappingModels::mentioned(table, text))
            .unwrap_or_default();
        ConverterModels::written(path, text, &models, self.environment_of())
            .map_err(|message| Self::failure(path, message))
    }

    fn load_models(&mut self, path: &Path) -> Result<(), Failure> {
        if self.table.is_none() {
            let table = ModelTable::read_realm(self.root, self.realm, self.extension)
                .map_err(|message| Self::failure(path, message))?;
            self.table = Some(table);
        }
        Ok(())
    }

    fn failure(path: &Path, message: String) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            message,
            None,
            source_span(&path.to_string_lossy(), 0, 0),
        ))
    }
}
