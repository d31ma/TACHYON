use crate::Failure;
use crate::model_sources::ModelSources;
use crate::route_types::{Model, ModelTable, WebEventModels};
use std::path::Path;

pub(super) struct KotlinModelOrder;

impl KotlinModelOrder {
    const NO_ORDER: &'static str = "        else -> null";

    pub(super) fn text(snapshot_root: &Path) -> Result<String, Failure> {
        let models = ModelSources::read(snapshot_root, "client", "kt")?;
        let declared = models
            .separate_files()
            .into_iter()
            .flat_map(|(_, source)| ModelTable::declared_models("kt", &source));
        let cases: String = WebEventModels::models()
            .into_iter()
            .chain(declared)
            .filter_map(|(name, model)| Self::case(&name, &model))
            .collect();
        Ok(Self::function(&cases))
    }

    fn case(name: &str, model: &Model) -> Option<String> {
        let Model::Record(record) = model else {
            return None;
        };
        let fields: Vec<String> = record
            .fields
            .iter()
            .map(|field| format!("\"{}\"", field.name))
            .collect();
        Some(format!(
            "        \"{name}\" -> listOf({})\n",
            fields.join(", ")
        ))
    }

    fn function(cases: &str) -> String {
        format!(
            "\nfun tacModelOrder(model: String): List<String>? =\n    when (model) {{\n{cases}{}\n    }}\n",
            Self::NO_ORDER
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::KotlinModelOrder;

    #[test]
    fn a_kotlin_model_keeps_the_order_its_constructor_declares() {
        let root = tempfile::tempdir().expect("project");
        let models = root.path().join("client/models");
        std::fs::create_dir_all(&models).expect("models");
        std::fs::write(
            models.join("Tick.kt"),
            "data class Tick(val sku: String, val level: Int)\n",
        )
        .expect("model");
        let text = KotlinModelOrder::text(root.path()).expect("order");
        assert!(
            text.contains("        \"Tick\" -> listOf(\"sku\", \"level\")\n"),
            "{text}"
        );
        assert!(
            text.contains("        \"ApplicationStateChanged\" -> listOf(\"state\")\n"),
            "{text}"
        );
        assert!(text.ends_with("        else -> null\n    }\n"), "{text}");
    }
}
