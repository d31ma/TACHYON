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
        let realm = if environment == Self::TAC {
            Self::CLIENT
        } else {
            Self::SERVER
        };
        Self {
            root,
            realm,
            extension,
            table: None,
        }
    }

    pub(crate) fn written(&mut self, path: &Path, text: String) -> Result<String, Failure> {
        let in_converters = path
            .iter()
            .nth(1)
            .is_some_and(|folder| folder == Self::FOLDER);
        if !in_converters {
            return Ok(text);
        }
        let failed = |message: String| Self::failure(path, message);
        if self.table.is_none() {
            let table =
                ModelTable::read_realm(self.root, self.realm, self.extension).map_err(failed)?;
            self.table = Some(table);
        }
        let models = self
            .table
            .as_ref()
            .map(|table| MappingModels::mentioned(table, &text))
            .unwrap_or_default();
        let written = ConverterModels::written(
            path,
            &text,
            &models,
            if self.realm == Self::CLIENT {
                "tac"
            } else {
                "yon"
            },
        )
        .map_err(failed)?;
        Ok(written.unwrap_or(text))
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
