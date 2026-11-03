use super::{EvalCall, SafetyUtils};
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct EvalCheck;

impl EvalCheck {
    const CODE: u16 = 1325;
    const HELP: &'static str = "Code that runs a text as code runs whatever an attacker can put into that text. Call the function or method you mean directly, or choose it from a fixed list with a switch (ADR 0068).";

    pub(crate) fn diagnostics<'a>(
        files: impl Iterator<Item = (&'a Path, &'a [u8])>,
    ) -> Vec<Diagnostic> {
        SafetyUtils::each_file(files, Self::file_diagnostics)
    }

    fn file_diagnostics(path: &Path, bytes: &[u8]) -> Vec<Diagnostic> {
        let extension = Self::extension(path);
        let source = super::CodeText::source(path, bytes);
        let code = super::CodeText::of(extension, source);
        Self::code_diagnostics(path, extension, &code)
    }

    fn code_diagnostics(path: &Path, extension: &str, code: &str) -> Vec<Diagnostic> {
        Self::calls_for(extension)
            .iter()
            .filter_map(|call| Self::call_diagnostic(path, code, call))
            .collect()
    }

    fn calls_for(extension: &str) -> Vec<EvalCall> {
        EvalCall::for_extension(extension)
            .into_iter()
            .filter(|call| !call.word.is_empty())
            .collect()
    }

    fn call_diagnostic(path: &Path, code: &str, call: &EvalCall) -> Option<Diagnostic> {
        let at = call.found_in(code)?;
        Some(Self::diagnostic(path, code, at, call))
    }

    fn extension(path: &Path) -> &str {
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("")
    }

    fn diagnostic(path: &Path, code: &str, at: usize, call: &EvalCall) -> Diagnostic {
        let portable = SafetyUtils::portable(path);
        let message = format!(
            "'{portable}' line {} runs text as code with {}. Tachyon refuses eval and similar calls in every language.",
            SafetyUtils::line_at(code, at),
            call.word.trim_start_matches('.')
        );
        SafetyUtils::refusal(Self::CODE, message, Self::HELP, (&portable, at))
    }
}
