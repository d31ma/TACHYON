use crate::Failure;
use crate::annotation_rules::AnnotationRules;
use crate::failure::{diagnostic, source_span};
use crate::model_layout::ModelBlocks;
use std::ffi::OsStr;
use std::path::{Component, Path};

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub(crate) struct FileNameCheck<'a> {
    path: &'a Path,
    realm: &'a str,
    folder: &'a str,
    stem: &'a str,
    extension: &'a str,
}

impl<'a> FileNameCheck<'a> {
    pub(crate) const CODE: u16 = 1318;
    const REALMS: [&'static str; 2] = ["client", "server"];
    const FREE_FOLDERS: [&'static str; 4] = ["pages", "components", "shared", "workers"];
    const MODELS: &'static str = "models";
    const TYPE_LANGUAGES: [&'static str; 2] = ["go", "rs"];
    const RULE: &'static str =
        "A class file is named after its one class, with the same spelling, as in Java (ADR 0067).";

    pub(crate) fn check(path: &'a Path, source: &str) -> Result<(), Failure> {
        let file = Self::new(path);
        let names = file.names_if_checked(source)?;
        file.require_named_type(&names)
    }

    pub(crate) fn wanted_name(path: &'a Path, source: &str) -> Option<String> {
        let file = Self::new(path);
        let names = file.names_if_checked(source).unwrap_or_default();
        file.only_other_name(&names)
    }

    pub(crate) fn is_class_file(path: &'a Path) -> bool {
        Self::new(path).is_checked()
    }

    fn new(path: &'a Path) -> Self {
        Self {
            path,
            realm: Self::part(path, 0),
            folder: Self::part(path, 1),
            stem: Self::text(path.file_stem()),
            extension: Self::text(path.extension()),
        }
    }

    fn part(path: &'a Path, index: usize) -> &'a str {
        let folders = path.parent().unwrap_or(Path::new(""));
        let part = folders.components().nth(index);
        Self::text(part.map(Component::as_os_str))
    }

    fn text(value: Option<&'a OsStr>) -> &'a str {
        value.and_then(|found| found.to_str()).unwrap_or("")
    }

    fn is_checked(&self) -> bool {
        let checks = [
            Self::REALMS.contains(&self.realm),
            !self.folder.is_empty(),
            !Self::FREE_FOLDERS.contains(&self.folder),
            AnnotationRules::LANGUAGES.contains(&self.extension),
        ];
        !checks.contains(&false)
    }

    fn is_model(&self) -> bool {
        self.folder == Self::MODELS
    }

    fn names_if_checked(&self, source: &str) -> Result<Vec<String>, Failure> {
        let checked = self.is_checked().then(|| self.declared_names(source));
        checked.unwrap_or(Ok(Vec::new()))
    }

    fn declared_names(&self, source: &str) -> Result<Vec<String>, Failure> {
        let declared = super::declaration_names(self.path, source)?;
        let models: Vec<String> = self
            .model_names(source)
            .into_iter()
            .filter(|name| !declared.contains(name))
            .collect();
        Ok([declared, models].concat())
    }

    fn model_names(&self, source: &str) -> Vec<String> {
        let models = self
            .is_model()
            .then(|| ModelBlocks::names(self.path, source));
        models.and_then(Result::ok).unwrap_or_default()
    }

    fn only_other_name(&self, names: &[String]) -> Option<String> {
        let other = [names.len() == 1, !self.has_named_type(names)] == [true, true];
        other.then(|| names[0].clone())
    }

    fn has_named_type(&self, names: &[String]) -> bool {
        names.iter().any(|name| name == self.stem)
    }

    fn require_named_type(&self, names: &[String]) -> Result<(), Failure> {
        let passes = [names.is_empty(), self.has_named_type(names)].contains(&true);
        if passes {
            return Ok(());
        }
        Err(self.name_failure(&names[0]))
    }

    fn name_failure(&self, found: &str) -> Failure {
        let kind = self.kind_word();
        let message = format!(
            "The file name '{}' does not match the {kind} '{found}' that the file declares",
            self.stem
        );
        let help = format!(
            "Rename the file to '{found}.{}', or rename the {kind} to '{}'. {} `ty migrate fix --write` renames the file for you.",
            self.extension,
            self.stem,
            Self::RULE
        );
        self.failure(message, help)
    }

    fn kind_word(&self) -> &'static str {
        if Self::TYPE_LANGUAGES.contains(&self.extension) {
            return "type";
        }
        "class"
    }

    fn failure(&self, message: String, help: String) -> Failure {
        let file = self.path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(help),
            source_span(&file, 0, 0),
        ))
    }
}
