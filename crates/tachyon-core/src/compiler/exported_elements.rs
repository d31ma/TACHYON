use super::element_declaration::ElementDeclaration;
use super::element_events::ElementEvents;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::template::{
    ClientViewRenderer, ComponentDefinition, ComponentRegistry, ServiceRegistry,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) struct ExportedElements<'a> {
    renderer: ClientViewRenderer<'a>,
    components: &'a ComponentRegistry,
    services: &'a ServiceRegistry,
    project_root: &'a Path,
    has_styles: bool,
}

impl<'a> ExportedElements<'a> {
    const FOLDER: &'static str = ".tachyon/elements";
    const MANIFEST: &'static str = "client/manifest.json";
    const CODE: u16 = 1358;
    const RUNTIME_IMPORT: &'static str = "import { TacElements } from '../tac-client.js'\n";
    const MODULE_EXTENSION: &'static str = "js";
    const DECLARATION_EXTENSION: &'static str = "d.ts";

    pub(super) const fn new(
        renderer: ClientViewRenderer<'a>,
        components: &'a ComponentRegistry,
        services: &'a ServiceRegistry,
        project_root: &'a Path,
        has_styles: bool,
    ) -> Self {
        Self {
            renderer,
            components,
            services,
            project_root,
            has_styles,
        }
    }

    pub(super) fn files(
        &self,
        names: &[String],
        used_services: &mut BTreeSet<String>,
    ) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        let pairs = names
            .iter()
            .map(|name| self.element_files(name, used_services))
            .collect::<Result<Vec<_>, Failure>>()?;
        Ok(pairs.into_iter().flatten().collect())
    }

    fn element_files(
        &self,
        name: &str,
        used_services: &mut BTreeSet<String>,
    ) -> Result<[(PathBuf, Vec<u8>); 2], Failure> {
        let component = self.component(name)?;
        Self::check_hyphen(name)?;
        let definition = self.definition(name, component, used_services)?;
        let declaration = self.declaration(name, &definition);
        let module = format!("{}TacElements.define({definition})\n", Self::RUNTIME_IMPORT);
        Ok([
            (
                Self::path(name, Self::MODULE_EXTENSION),
                module.into_bytes(),
            ),
            (
                Self::path(name, Self::DECLARATION_EXTENSION),
                declaration.into_bytes(),
            ),
        ])
    }

    fn path(name: &str, extension: &str) -> PathBuf {
        PathBuf::from(format!("{}/{name}.{extension}", Self::FOLDER))
    }

    fn declaration(&self, name: &str, definition: &Value) -> String {
        let props = Self::props(definition);
        let service_names = Self::service_names(definition);
        let events = ElementEvents::collect(self.project_root, self.services, &service_names);
        ElementDeclaration::new(name, &props, &events).text()
    }

    fn props(definition: &Value) -> Vec<String> {
        Self::items(&definition["props"])
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect()
    }

    fn service_names(definition: &Value) -> Vec<String> {
        Self::items(&definition["node"]["services"])
            .iter()
            .filter_map(|service| service["name"].as_str())
            .map(String::from)
            .collect()
    }

    fn items(list: &Value) -> &[Value] {
        list.as_array().map_or(&[], Vec::as_slice)
    }

    fn component(&self, name: &str) -> Result<&'a ComponentDefinition, Failure> {
        self.components.get(name).ok_or_else(|| {
            Self::failure(
                format!("tac.elements names `{name}`, but no component has that tag."),
                String::from(
                    "Use the tag of a folder under client/components: client/components/shop/counter is `shop-counter`.",
                ),
            )
        })
    }

    fn check_hyphen(name: &str) -> Result<(), Failure> {
        name.contains('-').then_some(()).ok_or_else(|| {
            Self::failure(
                format!(
                    "`{name}` cannot be a custom element, because the name of a custom element must contain a hyphen."
                ),
                format!(
                    "Move client/components/{name} into a folder with two parts, such as client/components/app/{name}, and export `app-{name}`."
                ),
            )
        })
    }

    fn definition(
        &self,
        name: &str,
        component: &ComponentDefinition,
        used_services: &mut BTreeSet<String>,
    ) -> Result<Value, Failure> {
        let mut definition = self.renderer.element(name, component, used_services)?;
        definition["styles"] = Value::Bool(self.has_styles);
        Ok(definition)
    }

    fn failure(message: String, help: String) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(help),
            source_span(Self::MANIFEST, 0, 0),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "exported_elements_tests.rs"]
mod tests;
