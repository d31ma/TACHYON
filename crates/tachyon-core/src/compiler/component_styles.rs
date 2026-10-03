use super::component_style_boundary::ComponentStyleBoundary;
use super::portable_path::PortablePath;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::template::{ComponentDefinition, ComponentRegistry};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

pub(super) struct ComponentStyles;

impl ComponentStyles {
    const MAX_STYLE_BYTES: usize = 1_048_576;
    const SOURCE_NAME: &'static str = "tac.css";

    pub(super) fn collect(components: &ComponentRegistry) -> Result<String, Failure> {
        let mut bundle = String::new();
        components
            .names()
            .iter()
            .try_for_each(|name| Self::add_style(components, name, &mut bundle))?;
        Ok(bundle)
    }

    fn add_style(
        components: &ComponentRegistry,
        name: &str,
        bundle: &mut String,
    ) -> Result<(), Failure> {
        components.get(name).map_or(Ok(()), |component| {
            Self::add_component(name, component, bundle)
        })
    }

    fn add_component(
        name: &str,
        component: &ComponentDefinition,
        bundle: &mut String,
    ) -> Result<(), Failure> {
        let path = Path::new(&component.program().source_path).with_file_name(Self::SOURCE_NAME);
        let source_path = PortablePath::of(&path);
        component.style_path().map_or(Ok(()), |style| {
            Self::append_style(name, style, &source_path, bundle)
        })
    }

    fn append_style(
        name: &str,
        style: &Path,
        source_path: &str,
        bundle: &mut String,
    ) -> Result<(), Failure> {
        let source = Self::read(name, style, source_path)?;
        ComponentStyleBoundary::validate(source_path, &source)?;
        let _ = writeln!(
            bundle,
            "@scope ({}[{}=\"{name}\"]) to ({}[{}]) {{\n{}\n}}",
            ComponentRegistry::CONTAINER_TAG,
            ComponentRegistry::SCOPE_ATTRIBUTE,
            ComponentRegistry::CONTAINER_TAG,
            ComponentRegistry::SCOPE_ATTRIBUTE,
            source.trim_end()
        );
        Ok(())
    }

    fn read(name: &str, style: &Path, source_path: &str) -> Result<String, Failure> {
        let source = fs::read_to_string(style).map_err(|error| {
            Failure::one(diagnostic(
                1403,
                format!("Cannot read component stylesheet for '{name}': {error}"),
                Some(String::from(
                    "Check the stylesheet's permissions and encoding.",
                )),
                source_span(source_path, 0, 0),
            ))
        })?;
        if source.len() > Self::MAX_STYLE_BYTES {
            return Err(Failure::one(diagnostic(
                1403,
                format!("Component stylesheet for '{name}' exceeds the 1 MiB limit."),
                Some(String::from("Split the component or trim its stylesheet.")),
                source_span(source_path, 0, 0),
            )));
        }
        Ok(source)
    }
}
