use super::super::{Module, Prepared, module_failure, prepare};
use super::java_runtime_section::JavaRuntimeSection;
use crate::Failure;
use crate::behavior::HttpClientSource;
use std::path::Path;

#[derive(Clone, Default)]
pub(crate) struct JavaRuntimeSections {
    pub(super) configurations: JavaRuntimeSection,
    pub(super) entities: JavaRuntimeSection,
    pub(super) evictions: JavaRuntimeSection,
    pub(super) properties: JavaRuntimeSection,
    pub(super) values: JavaRuntimeSection,
    pub(super) selections: JavaRuntimeSection,
    pub(super) validations: JavaRuntimeSection,
}

impl JavaRuntimeSections {
    pub(crate) fn read(
        path: &Path,
        source: &str,
        environment: &str,
    ) -> Result<Option<Self>, Failure> {
        let rewritten = HttpClientSource::lowering_source(path, source, environment)?;
        let text = rewritten.as_deref().unwrap_or(source);
        prepare(path, text, environment)?
            .and_then(Self::annotated)
            .map(|module| Self::of(&module).map_err(|message| module_failure(path, text, message)))
            .transpose()
    }

    pub(crate) fn joined(self, other: &Self) -> Option<Self> {
        Some(Self {
            configurations: self.configurations.joined(&other.configurations)?,
            entities: self.entities.joined(&other.entities)?,
            evictions: self.evictions.joined(&other.evictions)?,
            properties: self.properties.joined(&other.properties)?,
            values: self.values.joined(&other.values)?,
            selections: self.selections.joined(&other.selections)?,
            validations: self.validations.joined(&other.validations)?,
        })
    }

    pub(crate) fn configured(&self) -> impl Iterator<Item = &str> {
        self.configurations.labels()
    }

    pub(super) fn fill_runtime(&self, module_id: &str, environment: &str) -> String {
        include_str!("../java_runtime.java")
            .replace("__MODULE__", module_id)
            .replace("__ENVIRONMENT__", environment)
            .replace("__CONFIGURATIONS__", &self.configurations.text())
            .replace("__ENTITIES__", &self.entities.text())
            .replace("__EVICTIONS__", &self.evictions.text())
            .replace("__PROPERTIES__", &self.properties.text())
            .replace("__VALUES__", &self.values.text())
            .replace("__SELECTIONS__", &self.selections.text())
            .replace("__VALIDATIONS__", &self.validations.text())
    }

    fn of(module: &Module) -> Result<Self, String> {
        super::reject_reserved_names(module)?;
        let mut sections = Self::default();
        module.types.iter().try_for_each(|type_declaration| {
            super::lower_type(module, type_declaration, &mut sections, &mut Vec::new())
        })?;
        Ok(sections)
    }

    fn annotated(prepared: Prepared) -> Option<Module> {
        match prepared {
            Prepared::Annotated(module) => Some(module),
            Prepared::Erased(_) => None,
        }
    }
}
