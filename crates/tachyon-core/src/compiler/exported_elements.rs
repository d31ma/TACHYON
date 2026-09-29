use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::template::{ClientViewRenderer, ComponentDefinition, ComponentRegistry};
use serde_json::Value;
use std::path::PathBuf;

pub(super) struct ExportedElements<'a> {
    renderer: ClientViewRenderer<'a>,
    components: &'a ComponentRegistry,
    has_styles: bool,
}

impl<'a> ExportedElements<'a> {
    const FOLDER: &'static str = ".tachyon/elements";
    const MANIFEST: &'static str = "client/manifest.json";
    const CODE: u16 = 1358;
    const RUNTIME_IMPORT: &'static str = "import { TacElements } from '../tac-client.js'\n";

    pub(super) const fn new(
        renderer: ClientViewRenderer<'a>,
        components: &'a ComponentRegistry,
        has_styles: bool,
    ) -> Self {
        Self {
            renderer,
            components,
            has_styles,
        }
    }

    pub(super) fn files(&self, names: &[String]) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        names.iter().map(|name| self.file(name)).collect()
    }

    fn file(&self, name: &str) -> Result<(PathBuf, Vec<u8>), Failure> {
        let component = self.component(name)?;
        Self::check_hyphen(name)?;
        let path = PathBuf::from(format!("{}/{name}.js", Self::FOLDER));
        Ok((path, self.module(name, component)?))
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

    fn module(&self, name: &str, component: &ComponentDefinition) -> Result<Vec<u8>, Failure> {
        let mut definition = self.renderer.element(name, component)?;
        definition["styles"] = Value::Bool(self.has_styles);
        let module = format!("{}TacElements.define({definition})\n", Self::RUNTIME_IMPORT);
        Ok(module.into_bytes())
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
