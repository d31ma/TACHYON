use super::{
    after_declaration_keyword, class_body_named, closing_parenthesis, rust_field_declarations,
};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::unicode_name::UnicodeName;
use std::path::Path;

pub(crate) struct KotlinHeader;

impl KotlinHeader {
    pub(crate) fn require_browser_constructor(
        path: &Path,
        source: &str,
        class: &str,
    ) -> Result<(), Failure> {
        let code = crate::annotation_layout::masked_source(path, source)?;
        Self::check_constructor(path, &code, class)
    }

    pub(crate) fn primary_types(
        path: &Path,
        source: &str,
        class: &str,
    ) -> Result<Vec<String>, Failure> {
        let code = crate::annotation_layout::masked_source(path, source)?;
        let header = Self::header(&code, class).unwrap_or_default();
        Self::parameter_types(path, Self::primary(header).unwrap_or_default())
    }

    fn check_constructor(path: &Path, source: &str, class: &str) -> Result<(), Failure> {
        let header = Self::header(source, class).unwrap_or_default();
        Self::require_public_constructor(path, header)?;
        let primary = Self::primary(header);
        let secondary = Self::secondary(source, class);
        let primary_default = primary.is_some_and(Self::defaults);
        let secondary_default = secondary
            .iter()
            .filter(|(accessible, _)| *accessible)
            .any(|(_, parameters)| Self::defaults(parameters));
        let usable = Self::usable([
            primary.is_some(),
            secondary.is_empty(),
            primary_default,
            secondary_default,
        ]);
        Self::require_usable(path, usable)
    }

    fn require_public_constructor(path: &Path, header: &str) -> Result<(), Failure> {
        if header.starts_with("private constructor") || header.starts_with("protected constructor")
        {
            return Err(Self::failure(
                path,
                "A Kotlin browser service cannot be activated through a private or protected primary constructor.",
            ));
        }
        Ok(())
    }

    fn usable(constructors: [bool; 4]) -> bool {
        matches!(
            constructors,
            [_, _, true, _] | [_, _, _, true] | [false, true, _, _]
        )
    }

    fn require_usable(path: &Path, usable: bool) -> Result<(), Failure> {
        if !usable {
            return Err(Self::failure(
                path,
                "The authored Kotlin browser service needs a usable no-argument constructor; internal generated provider constructors are not public component arguments.",
            ));
        }
        Ok(())
    }

    fn header<'a>(source: &'a str, class: &str) -> Option<&'a str> {
        source
            .lines()
            .filter_map(|line| after_declaration_keyword(line, "class"))
            .find(|after| UnicodeName::leading(after) == class)
            .map(|after| {
                let at = after.as_ptr() as usize - source.as_ptr() as usize + class.len();
                source[at..].trim_start()
            })
    }

    fn primary(header: &str) -> Option<&str> {
        let header = header
            .trim_start()
            .strip_prefix("public ")
            .unwrap_or(header.trim_start());
        let header = header
            .strip_prefix("constructor")
            .unwrap_or(header)
            .trim_start();
        header
            .starts_with('(')
            .then(|| closing_parenthesis(header).map(|close| &header[1..close]))
            .flatten()
    }

    fn defaults(parameters: &str) -> bool {
        parameters.trim().is_empty()
            || rust_field_declarations(parameters)
                .iter()
                .all(|parameter| parameter.contains('='))
    }

    fn secondary(source: &str, class: &str) -> Vec<(bool, String)> {
        let body = class_body_named(source, Some(class)).map_or("", |(_, body)| body);
        body.lines().filter_map(Self::secondary_header).collect()
    }

    fn secondary_header(line: &str) -> Option<(bool, String)> {
        let header = line.trim_start();
        let accessible = !header.starts_with("private ") && !header.starts_with("protected ");
        let header = Self::without_visibility(header).strip_prefix("constructor")?;
        Self::primary(header.trim_start()).map(|parameters| (accessible, parameters.to_owned()))
    }

    fn without_visibility(header: &str) -> &str {
        ["public ", "internal ", "private ", "protected "]
            .iter()
            .find_map(|prefix| header.strip_prefix(prefix))
            .unwrap_or(header)
    }

    fn parameter_types(path: &Path, parameters: &str) -> Result<Vec<String>, Failure> {
        rust_field_declarations(parameters)
            .iter()
            .filter(|parameter| !parameter.trim().is_empty())
            .map(|parameter| Self::parameter_type(path, parameter))
            .collect()
    }

    fn parameter_type(path: &Path, parameter: &str) -> Result<String, Failure> {
        Self::public_property(path, parameter)?;
        let (_, kind) = parameter.split_once(':').ok_or_else(|| {
            Self::failure(
                path,
                "Kotlin model constructor properties need explicit types.",
            )
        })?;
        Ok(kind.split('=').next().unwrap_or_default().trim().to_owned())
    }

    fn public_property(path: &Path, parameter: &str) -> Result<(), Failure> {
        let parameter = parameter
            .trim()
            .strip_prefix("public ")
            .unwrap_or(parameter.trim());
        if !["val ", "var "]
            .iter()
            .any(|prefix| parameter.starts_with(prefix))
        {
            return Err(Self::failure(
                path,
                "A Kotlin browser DTO needs public primary constructor properties for its existing model codec.",
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
