use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::template::{ClientViewRenderer, ComponentRegistry};
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
        let mut files = Vec::new();
        for name in names {
            self.check(name)?;
            let path = PathBuf::from(format!("{}/{name}.js", Self::FOLDER));
            files.push((path, self.module(name)?));
        }
        Ok(files)
    }

    fn check(&self, name: &str) -> Result<(), Failure> {
        if self.components.get(name).is_none() {
            return Err(Self::failure(
                format!("tac.elements names `{name}`, but no component has that tag."),
                String::from(
                    "Use the tag of a folder under client/components: client/components/shop/counter is `shop-counter`.",
                ),
            ));
        }
        if !name.contains('-') {
            return Err(Self::failure(
                format!(
                    "`{name}` cannot be a custom element, because the name of a custom element must contain a hyphen."
                ),
                format!(
                    "Move client/components/{name} into a folder with two parts, such as client/components/app/{name}, and export `app-{name}`."
                ),
            ));
        }
        Ok(())
    }

    fn module(&self, name: &str) -> Result<Vec<u8>, Failure> {
        let mut definition = self.renderer.element(name)?;
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
#[path = "exported_elements_tests.rs"]
mod tests;
