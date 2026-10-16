use super::ManualStep;
use crate::Failure;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct OneClassReport<'a> {
    relative: &'a str,
    source: &'a str,
}

impl<'a> OneClassReport<'a> {
    const ONE_CLASS_MESSAGE: &'static str =
        "An annotated or handler file holds one class and its imports";
    const MIDDLEWARE_NAME_MESSAGE: &'static str = "root middleware declares";
    const OTHER_CLASS_PREFIX: &'static str = "another ";
    const TYPE_EXTENSIONS: [&'static str; 2] = ["go", "rs"];

    pub(super) fn steps(
        relative: &'a str,
        source: &'a str,
        skip_middleware_name: bool,
    ) -> Vec<ManualStep> {
        let report = Self { relative, source };
        let checked = crate::annotation_layout::has_primary_role(Path::new(relative), source);
        checked
            .err()
            .map(|failure| report.failure_steps(&failure, skip_middleware_name))
            .unwrap_or_default()
    }

    fn failure_steps(&self, failure: &Failure, skip_middleware_name: bool) -> Vec<ManualStep> {
        failure
            .diagnostics()
            .iter()
            .filter(|diagnostic| Self::needs_step(diagnostic, skip_middleware_name))
            .map(|diagnostic| self.step(diagnostic))
            .collect()
    }

    fn needs_step(diagnostic: &Diagnostic, skip_middleware_name: bool) -> bool {
        let names_middleware = diagnostic.message.contains(Self::MIDDLEWARE_NAME_MESSAGE);
        let skipped = [skip_middleware_name, names_middleware] == [true, true];
        [diagnostic.code.number() == 1310, skipped] == [true, false]
    }

    fn step(&self, diagnostic: &Diagnostic) -> ManualStep {
        let at = diagnostic
            .spans
            .first()
            .map_or(0, |span| usize::try_from(span.start).unwrap_or(0));
        let line = self.source[..at.min(self.source.len())]
            .matches('\n')
            .count()
            + 1;
        let mut action = diagnostic.help.clone().unwrap_or_default();
        let splits_class = diagnostic.message.starts_with(Self::ONE_CLASS_MESSAGE);
        let split = splits_class.then(|| format!(" {}", self.split(&diagnostic.message, line)));
        action.push_str(&split.unwrap_or_default());
        ManualStep::new(
            &format!("{}:{line}", self.relative),
            diagnostic.message.clone(),
            action,
        )
    }

    fn split(&self, message: &str, line: usize) -> String {
        let path = Path::new(self.relative);
        let names =
            crate::annotation_layout::declaration_names(path, self.source).unwrap_or_default();
        let kept = self.kept_name(&names);
        let kind = self.type_word();
        let mut text = format!("Split: keep {kind} {kept} in {}.", self.relative);
        for name in names.iter().filter(|name| **name != kept) {
            text.push(' ');
            text.push_str(&self.move_type(name));
        }
        let found = message.rsplit("found ").next().unwrap_or("");
        text.push_str(&Self::shared_code_advice(found, line, &kept, self.realm()));
        text
    }

    fn shared_code_advice(found: &str, line: usize, kept: &str, realm: &str) -> String {
        if found.starts_with(Self::OTHER_CLASS_PREFIX) {
            return String::new();
        }
        format!(
            " Move the code at line {line} ({found}) into '{realm}/shared/', or make it a method of {kept}."
        )
    }

    fn kept_name(&self, names: &[String]) -> String {
        let entry_name = self.entry_name();
        let names_entry = names.iter().any(|name| name == entry_name);
        let first = names.first().filter(|_| !names_entry);
        first.map_or_else(|| String::from(entry_name), String::clone)
    }

    fn entry_name(&self) -> &'a str {
        let path = Path::new(self.relative);
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        match stem {
            "middleware" if !self.relative.contains('/') => "Middleware",
            "yon" => "Handler",
            _ if self.relative.starts_with("server/workers/") => "Handler",
            _ => stem,
        }
    }

    fn move_type(&self, name: &str) -> String {
        let realm = self.realm();
        let extension = Path::new(self.relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        format!(
            "Move {} {name} to '{realm}/models/{name}.{extension}' if it only holds data, to its own file in its role folder if it has a Tachyon annotation, or to '{realm}/shared/{name}.{extension}' otherwise.",
            self.type_word()
        )
    }

    fn realm(&self) -> &'static str {
        if self.relative.starts_with("client/") {
            return "client";
        }
        "server"
    }

    fn type_word(&self) -> &'static str {
        let extension = Path::new(self.relative)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if Self::TYPE_EXTENSIONS.contains(&extension) {
            return "type";
        }
        "class"
    }
}
