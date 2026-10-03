use super::java_enclosure::JavaEnclosure;
use super::java_model::JavaModel;
use super::java_model_kind::JavaModelKind;
use super::java_model_reader::JavaModelReader;
use super::java_model_table::JavaModelTable;
use super::java_value_type::JavaValueType;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(super) struct JavaTypeModels {
    models: Vec<JavaModel>,
    paths: BTreeMap<String, PathBuf>,
}

impl JavaTypeModels {
    pub(super) fn read(files: &[(PathBuf, String)]) -> Result<Self, Failure> {
        let types = Self::read_files(files)?;
        types.require_supported()?;
        Ok(types)
    }

    pub(super) fn values(&self) -> JavaValueType {
        JavaValueType::new(self.names())
    }

    pub(super) fn lines(&self) -> String {
        self.namespace()
    }

    pub(super) fn without_models(&self, path: &Path, source: &str) -> Result<String, Failure> {
        let names = self.names();
        let blocks = crate::model_layout::ModelBlocks::read(path, source)?;
        let mut bytes = source.as_bytes().to_vec();
        blocks
            .iter()
            .filter(|block| names.contains(&block.name))
            .for_each(|block| {
                crate::model_layout::JavaModelSchema::blank_bytes(
                    &mut bytes[block.start..block.end],
                );
            });
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn read_files(files: &[(PathBuf, String)]) -> Result<Self, Failure> {
        let mut types = Self {
            models: Vec::new(),
            paths: BTreeMap::new(),
        };
        files
            .iter()
            .try_for_each(|(path, source)| types.add_file(path, source))?;
        Ok(types)
    }

    fn add_file(&mut self, path: &Path, source: &str) -> Result<(), Failure> {
        let declared = Self::read_file(path, source)?;
        declared
            .iter()
            .try_for_each(|model| self.register_name(path, &model.name))?;
        self.models.extend(declared);
        Ok(())
    }

    fn register_name(&mut self, path: &Path, name: &str) -> Result<(), Failure> {
        if self.paths.contains_key(name) {
            return Err(Failure::one(diagnostic(
                1405,
                format!("The Java browser model '{name}' is declared more than once."),
                None,
                source_span(&path.to_string_lossy(), 0, 1),
            )));
        }
        self.paths.insert(name.to_owned(), path.to_path_buf());
        Ok(())
    }

    fn read_file(path: &Path, source: &str) -> Result<Vec<JavaModel>, Failure> {
        let source = &crate::topics::Topics::blanked(source);
        crate::model_layout::validate(path, source)?;
        let plain = crate::model_layout::JavaModelSchema::blanked(path, source)?;
        let compiled = JavaEnclosure::model_source(&plain);
        Ok(JavaModelReader::models(&compiled))
    }

    fn require_supported(&self) -> Result<(), Failure> {
        let values = self.values();
        let table = JavaModelTable::new(&self.models, &values);
        let refusal = self.models.iter().find_map(|model| {
            table
                .refusal(model)
                .map(|message| self.model_failure(&model.name, message))
        });
        refusal.map_or(Ok(()), Err)
    }

    fn model_failure(&self, name: &str, message: String) -> Failure {
        let span = self
            .paths
            .get(name)
            .and_then(|path| source_span(&path.to_string_lossy(), 0, 1));
        Failure::one(diagnostic(1405, message, None, span))
    }

    fn names(&self) -> Vec<String> {
        self.models.iter().map(|model| model.name.clone()).collect()
    }

    fn namespace(&self) -> String {
        if self.models.is_empty() {
            return String::new();
        }
        format!(
            "  namespace __TachyonJavaModels {{\n{}  }}\n",
            self.model_lines()
        )
    }

    fn model_lines(&self) -> String {
        let values = self.values();
        self.models
            .iter()
            .map(|model| Self::model_line(model, &values))
            .collect()
    }

    fn model_line(model: &JavaModel, values: &JavaValueType) -> String {
        let render = Self::model_renderer(&model.kind);
        render(model, values)
    }

    fn model_renderer(kind: &JavaModelKind) -> fn(&JavaModel, &JavaValueType) -> String {
        match kind {
            JavaModelKind::Enumeration { .. } => Self::enumeration_model,
            _ => Self::fields,
        }
    }

    fn enumeration_model(model: &JavaModel, _: &JavaValueType) -> String {
        match &model.kind {
            JavaModelKind::Enumeration { constants } => Self::enumeration(&model.name, constants),
            _ => String::new(),
        }
    }

    fn enumeration(name: &str, constants: &[String]) -> String {
        let choices = constants
            .iter()
            .map(|name| serde_json::to_string(name).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" | ");
        let choices = Self::enumeration_choices(&choices);
        format!("    type {name} = {choices};\n")
    }

    fn enumeration_choices(choices: &str) -> &str {
        if choices.is_empty() { "never" } else { choices }
    }

    fn fields(model: &JavaModel, values: &JavaValueType) -> String {
        let read_only = Self::modifier(&model.kind);
        let mut fields = String::new();
        for (kind, name) in &model.fields {
            let field = serde_json::to_string(name).unwrap_or_default();
            let value_type = values
                .typescript(kind)
                .unwrap_or_else(|| String::from("never"));
            let _ = writeln!(fields, "      {read_only}{field}: {value_type};");
        }
        format!("    interface {} {{\n{fields}    }}\n", model.name)
    }

    fn modifier(kind: &JavaModelKind) -> &'static str {
        if *kind == JavaModelKind::Record {
            "readonly "
        } else {
            ""
        }
    }

    pub(super) fn require_service_name(&self, path: &Path, name: &str) -> Result<(), Failure> {
        self.check_service_name(path, name)
    }

    fn check_service_name(&self, path: &Path, name: &str) -> Result<(), Failure> {
        if self.models.iter().any(|model| model.name == name) {
            return Err(Failure::one(diagnostic(
                1405,
                format!("The Java model '{name}' conflicts with its service class"),
                None,
                source_span(&path.to_string_lossy(), 0, 1),
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::JavaTypeModels;
    use std::path::PathBuf;

    #[test]
    fn a_topic_model_is_read_as_a_plain_record() {
        let files = [(
            PathBuf::from("client/models/InventoryRecord.java"),
            String::from("@Topic\nrecord InventoryRecord(String sku, long level) {}\n"),
        )];
        JavaTypeModels::read(&files).expect("a Topic record is a model");
    }
}
