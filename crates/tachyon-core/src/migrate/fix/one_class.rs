use super::ManualStep;
use std::fmt::Write as _;
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

    pub(super) fn steps(
        relative: &'a str,
        source: &'a str,
        skip_middleware_name: bool,
    ) -> Vec<ManualStep> {
        let report = Self { relative, source };
        let Err(failure) = crate::annotation_layout::has_primary_role(Path::new(relative), source)
        else {
            return Vec::new();
        };
        let mut steps = Vec::new();
        for diagnostic in failure.diagnostics() {
            let names_middleware = diagnostic.message.contains(Self::MIDDLEWARE_NAME_MESSAGE);
            if diagnostic.code.number() != 1310 || (skip_middleware_name && names_middleware) {
                continue;
            }
            steps.push(report.step(diagnostic));
        }
        steps
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
        if diagnostic.message.starts_with(Self::ONE_CLASS_MESSAGE) {
            action.push(' ');
            action.push_str(&self.split(&diagnostic.message, line));
        }
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
        for name in &names {
            if name != &kept {
                text.push(' ');
                text.push_str(&self.move_type(name));
            }
        }
        let found = message.rsplit("found ").next().unwrap_or("");
        if !found.starts_with(Self::OTHER_CLASS_PREFIX) {
            let realm = self.realm();
            let _ = write!(
                text,
                " Move the code at line {line} ({found}) into '{realm}/shared/', or make it a method of {kept}."
            );
        }
        text
    }

    fn kept_name(&self, names: &[String]) -> String {
        let path = Path::new(self.relative);
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let entry_name = match stem {
            "middleware" if !self.relative.contains('/') => "Middleware",
            "yon" => "Handler",
            _ if self.relative.starts_with("server/workers/") => "Handler",
            _ => stem,
        };
        if names.iter().any(|name| name == entry_name) || names.is_empty() {
            return String::from(entry_name);
        }
        names[0].clone()
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
            .and_then(|value| value.to_str());
        if matches!(extension, Some("go" | "rs")) {
            return "type";
        }
        "class"
    }
}
