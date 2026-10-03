#![allow(clippy::needless_bitwise_bool)]

use super::cohesion::UtilityShape;
use super::rust_utility_exports::RustUtilityExports;
use super::{Failure, Input, Parser, Tokens};
use crate::annotation_layout::ClassFileName;
use std::path::Path;

#[cfg(test)]
mod tests;

pub(crate) struct UtilityLayout;

impl UtilityLayout {
    pub(crate) const CLIENT_ROOT: &'static str = "client/shared/utils";
    pub(crate) const REMOVED_CLIENT_ROOT: &'static str = "client/utils";
    const FOLDER_CODE: u16 = 1310;
    const SUFFIX: &'static str = "Utils";
    const NAME_HELP: &'static str = "Name the file and its class <Topic>Utils, such as TextUtils, in its language filename style; Rust accepts text_utils.rs or legacy TextUtils.rs (ADR 0064).";
    const MARKER_HELP: &'static str = "Delete the annotation. Tachyon gives every utility class to the code of its realm without an annotation or an import (ADR 0064).";

    pub(crate) fn is_utility(path: &Path) -> bool {
        Self::utility_folder(path)
    }

    pub(crate) fn is_client_utility(path: &Path) -> bool {
        Self::client_folder(path)
    }

    fn utility_folder(path: &Path) -> bool {
        [Self::client_folder(path), path.starts_with("server/utils")].contains(&true)
    }

    fn client_folder(path: &Path) -> bool {
        path.starts_with(Self::CLIENT_ROOT)
    }

    pub(crate) fn reject_removed_root(path: &Path) -> Result<(), Failure> {
        let Ok(tail) = path.strip_prefix(Self::REMOVED_CLIENT_ROOT) else {
            return Ok(());
        };
        let portable = path.to_string_lossy().replace('\\', "/");
        let target = Path::new(Self::CLIENT_ROOT)
            .join(tail)
            .to_string_lossy()
            .replace('\\', "/");
        Err(Failure::one(crate::failure::diagnostic(
            Self::FOLDER_CODE,
            format!(
                "'{portable}' is inside client/utils; Tac utility classes live only in client/shared/utils"
            ),
            Some(format!(
                "Move the file to {target}; nested folders are allowed, and Yon keeps server/utils (ADR 0064). `ty migrate fix --write` moves every file in client/utils and changes the paths that import them."
            )),
            crate::failure::source_span(&portable, 0, 0),
        )))
    }

    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        let checked = Self::is_utility(path).then(|| Self::check_utility(path, source));
        checked.unwrap_or(Ok(()))
    }

    pub(crate) fn native_rust_exports(
        path: &Path,
        source: &str,
    ) -> Result<(String, bool), Failure> {
        RustUtilityExports::source(path, source)
    }

    fn check_utility(path: &Path, source: &str) -> Result<(), Failure> {
        let input = Input::new(path, source);
        let stem = Self::stem(path);
        Self::require(
            Self::is_utility_file(path, stem),
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

    fn is_utility_file(path: &Path, stem: &str) -> bool {
        let rust = path.extension().is_some_and(|extension| extension == "rs");
        let snake = [rust, stem.len() > 6, stem.ends_with("_utils")]
            .iter()
            .all(|value| *value);
        [Self::is_utility_name(stem), snake].contains(&true)
    }

    fn is_utility_name(stem: &str) -> bool {
        (stem.len() > Self::SUFFIX.len()) & stem.ends_with(Self::SUFFIX)
    }

    fn check_one_class(input: &Input<'_>, tokens: &Tokens<'_>, stem: &str) -> Result<(), Failure> {
        let mut parser = Parser::new(input, tokens);
        parser.read()?;
        let named = parser.declared.as_deref().is_some_and(|name| {
            [
                Self::is_utility_name(name),
                ClassFileName::matches(input.path, name),
            ]
            .iter()
            .all(|value| *value)
        });
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
