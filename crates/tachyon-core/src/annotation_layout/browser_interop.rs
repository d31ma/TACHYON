use crate::Failure;
use crate::failure::{diagnostic, source_span};
use interop_finder::InteropFinder;
use interop_rules::InteropRules;
use std::ops::Range;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

mod browser_provider;
mod browser_providers;
mod interop_finder;
mod interop_rules;
mod native_web_apis;
mod rust_known_model_paths;

#[cfg(test)]
mod rust_known_model_paths_tests;

pub(crate) use native_web_apis::NativeWebApis;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod preference_provider_tests;

#[cfg(test)]
mod document_title_provider_tests;

pub(crate) struct BrowserInteropCheck;

impl BrowserInteropCheck {
    const CODE: u16 = 1317;
    const HELP: &'static str = "Browser work lives in a JavaScript or TypeScript service. That service publishes a typed event, and this service receives it with @OnEvent. See /docs/native-services";

    pub(crate) fn check_file(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        let source = String::from_utf8_lossy(bytes);
        let problems = Self::problems(path, &source);
        Self::refuse(problems)
    }

    pub(crate) fn problems(path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut problems = InteropRules::for_path(path)
            .map(|rules| Self::problems_with(path, source, rules))
            .unwrap_or_default();
        problems.extend(browser_providers::BrowserProviders::browser(path, source));
        problems
    }

    fn problems_with(path: &Path, source: &str, rules: &InteropRules) -> Vec<Diagnostic> {
        let finder = InteropFinder::new(rules.extension, source);
        finder
            .uses(rules)
            .into_iter()
            .map(|range| Self::problem(path, source, rules.language, range))
            .collect()
    }

    fn problem(path: &Path, source: &str, language: &str, range: Range<usize>) -> Diagnostic {
        let message = format!(
            "{language} services cannot call browser APIs. Put this code in a JS or TS service and send the result to this service with an event."
        );
        let help = format!("Remove `{}`. {}", &source[range.clone()], Self::HELP);
        let file = path.to_string_lossy().replace('\\', "/");
        diagnostic(
            Self::CODE,
            message,
            Some(help),
            source_span(&file, range.start, range.end),
        )
    }

    fn refuse(problems: Vec<Diagnostic>) -> Result<(), Failure> {
        if problems.is_empty() {
            return Ok(());
        }
        Err(Failure::new(problems))
    }
}
