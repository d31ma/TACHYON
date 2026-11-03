use super::kotlin_type_values::KotlinTypeValues;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::route_types::{Model, ModelTable, Shape};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) struct KotlinTypeModels {
    models: BTreeMap<String, (PathBuf, Model)>,
}

impl KotlinTypeModels {
    pub(super) fn read(files: &[(PathBuf, String)]) -> Result<Self, Failure> {
        let models = Self::read_files(files)?;
        models.require_supported(files)?;
        Ok(models)
    }

    pub(super) fn values(&self) -> KotlinTypeValues {
        KotlinTypeValues::new(self.models.keys().cloned().collect())
    }

    pub(super) fn lines(&self) -> String {
        self.namespace()
    }

    pub(super) fn require_service_name(&self, path: &Path, class: &str) -> Result<(), Failure> {
        self.distinct_service(path, class)
    }

    fn read_files(files: &[(PathBuf, String)]) -> Result<Self, Failure> {
        let mut types = Self {
            models: BTreeMap::new(),
        };
        files
            .iter()
            .try_for_each(|(path, source)| types.add_file(path, source))?;
        Ok(types)
    }

    fn add_file(&mut self, path: &Path, source: &str) -> Result<(), Failure> {
        let source = &crate::topics::Topics::blanked(source);
        crate::model_layout::validate(path, source)?;
        let schema = crate::behavior::ModelSchema::plain(path, source)?;
        crate::behavior::ModelSchema::require_realm(path, source, schema.is_some())?;
        if schema.is_some() {
            return Ok(());
        }
        ModelTable::declared_models("kt", source)
            .into_iter()
            .try_for_each(|(name, model)| self.insert(path, name, model))
    }

    fn insert(&mut self, path: &Path, name: String, model: Model) -> Result<(), Failure> {
        if self.models.contains_key(&name) {
            return Err(Self::failure(
                path,
                &format!("The Kotlin browser model '{name}' is declared more than once."),
            ));
        }
        self.models.insert(name, (path.to_path_buf(), model));
        Ok(())
    }

    fn require_supported(&self, files: &[(PathBuf, String)]) -> Result<(), Failure> {
        let values = self.values();
        self.models
            .iter()
            .try_for_each(|(name, (path, model))| Self::check_model(path, name, model, &values))?;
        files
            .iter()
            .try_for_each(|(path, source)| self.check_declared_fields(path, source))
    }

    fn check_declared_fields(&self, path: &Path, source: &str) -> Result<(), Failure> {
        if crate::annotation_layout::SchemaMarkers::contains(path, source)? {
            return Ok(());
        }
        let values = self.values();
        ModelTable::declared_models("kt", source)
            .iter()
            .try_for_each(|(name, model)| {
                Self::require_concrete(path, source, name, model)?;
                let types = crate::companion::KotlinHeader::primary_types(path, source, name)?;
                Self::require_primary_fields(path, model, types.len())?;
                types.iter().try_for_each(|kind| {
                    values
                        .shape(kind)
                        .map(|_| ())
                        .map_err(|message| Self::failure(path, &message))
                })
            })
    }

    fn require_concrete(
        path: &Path,
        source: &str,
        name: &str,
        model: &Model,
    ) -> Result<(), Failure> {
        let code = crate::annotation_layout::masked_source(path, source)?;
        let enumeration = super::kotlin_model_readers::KotlinModelReaders::enum_names(&code)
            .iter()
            .any(|declared| declared == name);
        match model {
            Model::Alias(_) if !enumeration => Err(Self::failure(
                path,
                "Kotlin type aliases need a concrete browser model codec before composed typing.",
            )),
            _ => Ok(()),
        }
    }

    fn require_primary_fields(path: &Path, model: &Model, count: usize) -> Result<(), Failure> {
        match model {
            Model::Record(record) if record.fields.len() != count => Err(Self::failure(
                path,
                "Kotlin browser DTO fields must be public primary constructor properties accepted by its existing reader.",
            )),
            _ => Ok(()),
        }
    }

    fn check_model(
        path: &Path,
        name: &str,
        model: &Model,
        values: &KotlinTypeValues,
    ) -> Result<(), Failure> {
        match model {
            Model::Record(record)
                if record.open || !record.bases.is_empty() || record.unreadable.is_some() =>
            {
                Err(Self::failure(
                    path,
                    &format!(
                        "Kotlin model '{name}' needs a readable closed data codec without inherited fields."
                    ),
                ))
            }
            Model::Record(record) => Self::check_fields(path, &record.fields, values),
            Model::Alias(Shape::Text) => Ok(()),
            Model::Alias(_) => Err(Self::failure(
                path,
                &format!(
                    "Kotlin model '{name}' needs an existing concrete data or enum codec; aliases are not browser constructors."
                ),
            )),
        }
    }

    fn check_fields(
        path: &Path,
        fields: &[crate::route_types::ModelField],
        values: &KotlinTypeValues,
    ) -> Result<(), Failure> {
        fields
            .iter()
            .try_for_each(|field| Self::check_shape(path, &field.shape, values))
    }

    fn check_shape(path: &Path, shape: &Shape, values: &KotlinTypeValues) -> Result<(), Failure> {
        match shape {
            Shape::Model(name) => values
                .shape(name)
                .map(|_| ())
                .map_err(|message| Self::failure(path, &message)),
            Shape::Optional(inner) | Shape::List(inner) => Self::check_shape(path, inner, values),
            _ => Ok(()),
        }
    }

    fn namespace(&self) -> String {
        self.nonempty()
            .map(Self::namespace_lines)
            .unwrap_or_default()
    }

    fn nonempty(&self) -> Option<&Self> {
        (!self.models.is_empty()).then_some(self)
    }

    fn namespace_lines(&self) -> String {
        let lines: String = self
            .models
            .iter()
            .map(|(name, (_, model))| Self::model_line(name, model))
            .collect();
        format!("  namespace __TachyonKotlinModels {{\n{lines}  }}\n")
    }

    fn model_line(name: &str, model: &Model) -> String {
        match model {
            Model::Alias(Shape::Text) => format!("    type {name} = string;\n"),
            Model::Record(record) => Self::record_line(name, &record.fields),
            Model::Alias(_) => String::new(),
        }
    }

    fn record_line(name: &str, fields: &[crate::route_types::ModelField]) -> String {
        let fields: String = fields.iter().map(Self::field_line).collect();
        format!("    interface {name} {{\n{fields}    }}\n")
    }

    fn field_line(field: &crate::route_types::ModelField) -> String {
        let name = serde_json::to_string(&field.name).unwrap_or_default();
        format!(
            "      {name}: {};\n",
            KotlinTypeValues::rendered(&field.shape)
        )
    }

    fn distinct_service(&self, path: &Path, class: &str) -> Result<(), Failure> {
        if self.models.contains_key(class) {
            return Err(Self::failure(
                path,
                "A Kotlin browser service cannot share its class name with a client model.",
            ));
        }
        Ok(())
    }

    fn failure(path: &Path, message: &str) -> Failure {
        Failure::one(diagnostic(
            1405,
            message,
            None,
            source_span(&path.to_string_lossy(), 0, 1),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::KotlinTypeModels;
    use std::path::PathBuf;

    #[test]
    fn a_topic_model_is_read_as_a_plain_data_class() {
        let files = [(
            PathBuf::from("client/models/InventoryRecord.kt"),
            String::from("@Topic\ndata class InventoryRecord(val sku: String, val level: Long)\n"),
        )];
        KotlinTypeModels::read(&files).expect("a Topic data class is a model");
    }
}
