#![allow(clippy::needless_bitwise_bool)]

use super::cohesion::UtilityShape;
use super::{Failure, Input, Parser, Tokens};
use std::path::Path;

#[cfg(test)]
mod tests;

pub(crate) struct UtilityLayout;

impl UtilityLayout {
    const FOLDERS: [&'static str; 2] = ["client/utils", "server/utils"];
    const SUFFIX: &'static str = "Utils";
    const NAME_HELP: &'static str = "Name the file and its class <Topic>Utils, such as TextUtils, with the same spelling (ADR 0064).";
    const MARKER_HELP: &'static str = "Delete the annotation. Tachyon gives every utility class to the code of its realm without an annotation or an import (ADR 0064).";

    pub(crate) fn is_utility(path: &Path) -> bool {
        Self::FOLDERS.iter().any(|folder| path.starts_with(folder))
    }

    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        let checked = Self::is_utility(path).then(|| Self::check_utility(path, source));
        checked.unwrap_or(Ok(()))
    }

    fn check_utility(path: &Path, source: &str) -> Result<(), Failure> {
        let input = Input::new(path, source);
        let stem = Self::stem(path);
        Self::require(
            Self::is_utility_name(stem),
            &input,
            format!("The utility file name '{stem}' does not end with Utils"),
            Self::NAME_HELP,
        )?;
        let tokens = Tokens::read(&input)?;
        tokens.check_complete(&input)?;
        Self::require(
            !tokens.has_framework_marker(),
            &input,
            "A utility class has no Tachyon annotation",
            Self::MARKER_HELP,
        )?;
        Self::check_one_class(&input, &tokens, stem)?;
        Self::check_shape(path, source)
    }

    fn stem(path: &Path) -> &str {
        path.file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("")
    }

    fn is_utility_name(stem: &str) -> bool {
        (stem.len() > Self::SUFFIX.len()) & stem.ends_with(Self::SUFFIX)
    }

    fn check_one_class(input: &Input<'_>, tokens: &Tokens<'_>, stem: &str) -> Result<(), Failure> {
        let mut parser = Parser::new(input, tokens);
        parser.read()?;
        let named = parser.declared.as_deref() == Some(stem);
        Self::require(
            named,
            input,
            format!("A utility file declares one class named {stem}"),
            Self::NAME_HELP,
        )
    }

    fn check_shape(path: &Path, source: &str) -> Result<(), Failure> {
        let problems = UtilityShape::problems(path, source)?;
        Self::require_none(problems)
    }

    fn require(
        holds: bool,
        input: &Input<'_>,
        message: impl Into<String>,
        help: &str,
    ) -> Result<(), Failure> {
        if holds {
            return Ok(());
        }
        Err(input.failure(0, message, help))
    }

    fn require_none(problems: Vec<tachyon_diagnostics::Diagnostic>) -> Result<(), Failure> {
        if problems.is_empty() {
            return Ok(());
        }
        Err(Failure::new(problems))
    }
}
