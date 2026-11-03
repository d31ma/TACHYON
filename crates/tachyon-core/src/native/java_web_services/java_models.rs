use super::java_enclosure::JavaEnclosure;
use super::java_model::JavaModel;
use super::java_model_reader::JavaModelReader;
use super::java_model_table::JavaModelTable;
use super::java_value_type::JavaValueType;
use super::java_web_apis::JavaWebApis;
use crate::Failure;
use std::path::{Path, PathBuf};

pub(super) struct JavaModels {
    files: Vec<(PathBuf, Vec<JavaModel>, String)>,
    authored: crate::model_sources::ModelSources,
}

impl JavaModels {
    const REALM: &'static str = "client";
    const EXTENSION: &'static str = "java";

    pub(super) fn read(snapshot_root: &Path, authored: &str) -> Result<Self, Failure> {
        let sources =
            crate::model_sources::ModelSources::read(snapshot_root, Self::REALM, Self::EXTENSION)?;
        let mut files: Vec<(PathBuf, Vec<JavaModel>, String)> = sources
            .project_files()
            .into_iter()
            .map(Self::model_file)
            .collect::<Result<_, Failure>>()?;
        files.extend(JavaWebApis::events(authored).map(|(path, source)| {
            let models = JavaModelReader::models(&source);
            (path, models, source)
        }));
        Ok(Self {
            files,
            authored: sources,
        })
    }

    pub(super) fn behavior_source(&self, source: &str, class: &str) -> Result<String, Failure> {
        let failure = self.class_conflict(class).then(|| {
            super::super::host::native_tool_failure(
                1405,
                &format!("The Java model '{class}' conflicts with its service class"),
            )
        });
        failure.map_or_else(|| Ok(self.authored.java_behavior_program(source)), Err)
    }

    fn class_conflict(&self, class: &str) -> bool {
        self.models().iter().any(|model| model.name == class)
    }

    pub(super) fn without_declarations(
        &self,
        path: &Path,
        source: &str,
    ) -> Result<String, Failure> {
        let names: Vec<_> = self.models().into_iter().map(|model| model.name).collect();
        crate::model_layout::ModelBlocks::read(path, source)
            .map(|blocks| Self::blank_declarations(&names, &blocks, source))
    }

    fn blank_declarations(
        names: &[String],
        blocks: &[crate::model_layout::ModelBlock],
        source: &str,
    ) -> String {
        let mut bytes = source.as_bytes().to_vec();
        blocks
            .iter()
            .filter(|block| names.contains(&block.name))
            .for_each(|block| {
                crate::model_layout::JavaModelSchema::blank_bytes(
                    &mut bytes[block.start..block.end],
                );
            });
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn model_file(
        (path, source): (PathBuf, String),
    ) -> Result<(PathBuf, Vec<JavaModel>, String), Failure> {
        let blanked = crate::model_layout::JavaModelSchema::blanked(&path, &source)?;
        let compiled = JavaEnclosure::model_source(&blanked);
        let models = JavaModelReader::models(&compiled);
        Ok((path, models, compiled))
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

    pub(super) fn table(&self, values: &JavaValueType) -> String {
        let models = self.models();
        let (_, table) = JavaModelTable::new(&models, values).file();
        table
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::JavaModels;
    use std::path::Path;

    struct Fixtures;

    impl Fixtures {
        const PATH: &'static str = "client/models/OrderInput.java";
        const MODEL: &'static str = "@Checked\nclass OrderInput {\n    @NotEmpty\n    @Length({\"min\":1,\"max\":80})\n    String name;\n    @Required\n    @Min(1)\n    @Max(9)\n    Object level;\n}\n";
        const SERVICE: &'static str = "import models.OrderInput;\n@Service\nclass OrderService {\n    Object accept(@Check(OrderInput.class) Object order) { return order; }\n}\n";

        fn models(root: &Path) -> JavaModels {
            let path = root.join(Self::PATH);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("models");
            std::fs::write(path, Self::MODEL).expect("model");
            JavaModels::read(root, Self::SERVICE).expect("models read")
        }
    }

    #[test]
    fn checked_java_models_preserve_schema_and_compile_as_one_plain_dto() {
        let root = tempfile::tempdir().expect("project");
        let models = Fixtures::models(root.path());
        let path = Path::new("client/services/OrderService.java");
        let roles =
            crate::behavior::ServiceRoles::read(path, Fixtures::SERVICE).expect("service role");
        let authored = models
            .behavior_source(&roles.source, "OrderService")
            .expect("assembled");
        let lowered = crate::behavior::lower(path, &authored, "tac")
            .expect("lowered")
            .expect("schema");
        let service = models
            .without_declarations(path, &lowered)
            .expect("service source");
        let source = models.sources(&models.values()).expect("compiled models");
        let dto = source
            .iter()
            .find(|(path, _)| path.ends_with("OrderInput.java"))
            .expect("dto");
        assert!(!service.contains("class OrderInput"));
        assert!(service.contains("case \"OrderInput\": return new Rule[]"));
        for marker in [
            "@Checked",
            "@NotEmpty",
            "@Length",
            "@Required",
            "@Min",
            "@Max",
        ] {
            assert!(!dto.1.contains(marker), "{marker}: {}", dto.1);
        }
        assert_eq!(dto.1.lines().count(), Fixtures::MODEL.lines().count());
        assert_eq!(dto.1.len(), Fixtures::MODEL.len());
        assert!(dto.1.contains("String name;") && dto.1.contains("Object level;"));
    }

    #[test]
    fn a_java_model_cannot_shadow_its_service() {
        let root = tempfile::tempdir().expect("project");
        let models = Fixtures::models(root.path());
        let failure = models
            .behavior_source("@Service\nclass OrderInput {}", "OrderInput")
            .expect_err("conflicting model");
        assert!(
            failure
                .to_string()
                .contains("conflicts with its service class")
        );
    }
}
