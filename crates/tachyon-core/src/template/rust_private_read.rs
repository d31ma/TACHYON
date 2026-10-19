use super::{ServiceDefinition, ServiceRegistry};
use crate::failure::{diagnostic, source_span};
use crate::tac_web::WebLanguage;
use std::collections::BTreeSet;
use tachyon_diagnostics::Diagnostic;

pub(super) struct RustPrivateRead;

impl RustPrivateRead {
    const CODE: u16 = 1311;

    pub(super) fn found(
        services: &ServiceRegistry,
        reads: &BTreeSet<(String, String)>,
        source_path: &str,
    ) -> Vec<Diagnostic> {
        reads
            .iter()
            .filter_map(|read| Self::refused(services, read, source_path))
            .collect()
    }

    fn refused(
        services: &ServiceRegistry,
        (instance, member): &(String, String),
        source_path: &str,
    ) -> Option<Diagnostic> {
        let service = services.get(instance)?;
        let private = Self::private_methods(service)?;
        let name = private.iter().find(|name| Self::same(name, member))?;
        let class = service.class();
        let message = format!(
            "The view reads {instance}.{member}, but {name} is a private method of {class}."
        );
        let help = format!(
            "The page sees only the `pub` methods of a Rust service. Write `pub fn {name}` in {class}."
        );
        Some(diagnostic(
            Self::CODE,
            message,
            Some(help),
            source_span(source_path, 0, 0),
        ))
    }

    fn private_methods(service: &ServiceDefinition) -> Option<Vec<String>> {
        let (language, path) = service.web_assembly()?;
        let source = (language == WebLanguage::Rust)
            .then_some(path)
            .and_then(|path| std::fs::read_to_string(path).ok())?;
        Some(source.lines().filter_map(Self::private_method).collect())
    }

    fn private_method(line: &str) -> Option<String> {
        let line = line.trim();
        let rest = line
            .strip_prefix("async ")
            .unwrap_or(line)
            .strip_prefix("fn ")?;
        let name = rest.split('(').next()?.trim();
        let receiver = rest[name.len()..].trim_start();
        let takes_self = [
            receiver.starts_with("(&self"),
            receiver.starts_with("(&mut self"),
        ];
        takes_self.contains(&true).then(|| name.to_owned())
    }

    fn same(name: &str, member: &str) -> bool {
        name.replace('_', "")
            .eq_ignore_ascii_case(&member.replace('_', ""))
    }
}
