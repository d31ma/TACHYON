use crate::events::publications::Publications;
use crate::route_types::ModelTable;
use crate::template::ServiceRegistry;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct ElementEvents {
    pub(super) names: BTreeSet<String>,
    pub(super) models: Map<String, Value>,
}

impl ElementEvents {
    const REALM: &'static str = "client";
    const MODEL_LANGUAGES: [&'static str; 2] = ["ts", "js"];
    const DOCUMENTED_LANGUAGE: &'static str = "js";

    pub(super) fn collect(
        project_root: &Path,
        services: &ServiceRegistry,
        service_names: &[String],
    ) -> Self {
        let mut events = Self {
            names: BTreeSet::new(),
            models: Map::new(),
        };
        let scripts = service_names
            .iter()
            .filter_map(|name| services.get(name)?.script_path());
        for script in scripts {
            events.add_script(project_root, script);
        }
        events
    }

    fn add_script(&mut self, project_root: &Path, script: &Path) {
        let language = script.extension().and_then(|extension| extension.to_str());
        let Some(language) = language.filter(|found| Self::MODEL_LANGUAGES.contains(found)) else {
            return;
        };
        let source = std::fs::read_to_string(script).unwrap_or_default();
        let tables = Self::model_tables(project_root, language, &source);
        for name in Publications::event_names(language, &source) {
            self.add_models(&tables, &name);
            self.names.insert(name);
        }
    }

    fn add_models(&mut self, tables: &[ModelTable], name: &str) {
        let described = tables
            .iter()
            .find_map(|table| table.event_models(name).ok());
        if let Some(Value::Object(models)) = described {
            self.models.extend(models);
        }
    }

    fn model_tables(project_root: &Path, language: &str, source: &str) -> Vec<ModelTable> {
        let mut documented = ModelTable::default();
        if language == Self::DOCUMENTED_LANGUAGE {
            documented.add_documented(source);
        }
        let folders = Self::MODEL_LANGUAGES.iter().filter_map(|model_language| {
            ModelTable::read_realm(project_root, Self::REALM, model_language).ok()
        });
        std::iter::once(documented).chain(folders).collect()
    }
}
