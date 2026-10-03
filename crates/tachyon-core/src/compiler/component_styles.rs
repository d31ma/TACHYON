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
        let style = components
            .get(name)
            .and_then(ComponentDefinition::style_path);
        style.map_or(Ok(()), |style| Self::append_style(name, style, bundle))
    }

    fn append_style(name: &str, style: &Path, bundle: &mut String) -> Result<(), Failure> {
        let source = Self::read(name, style)?;
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

    fn read(name: &str, style: &Path) -> Result<String, Failure> {
        let source = fs::read_to_string(style).map_err(|error| {
            Failure::one(diagnostic(
                1403,
                format!("Cannot read component stylesheet for '{name}': {error}"),
                Some(String::from(
                    "Check the stylesheet's permissions and encoding.",
                )),
                source_span(&PortablePath::of(style), 0, 0),
            ))
        })?;
        if source.len() > Self::MAX_STYLE_BYTES {
            return Err(Failure::one(diagnostic(
                1403,
                format!("Component stylesheet for '{name}' exceeds the 1 MiB limit."),
                Some(String::from("Split the component or trim its stylesheet.")),
                source_span(&PortablePath::of(style), 0, 0),
            )));
        }
        Ok(source)
    }
}
