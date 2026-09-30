use super::{CodeText, SafetyUtils};
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct ServiceReachCheck;

impl ServiceReachCheck {
    const GLOBAL_CODE: u16 = 1360;
    const FETCH_CODE: u16 = 1361;
    const SERVICES: &'static str = "client/services/";
    const SCRIPTS: [&'static str; 3] = ["js", "mjs", "ts"];
    const GLOBAL: &'static str = "globalThis";
    const FETCH: &'static str = "fetch";
    const FETCH_OWNERS: [&'static str; 2] = ["window.", "self."];
    const GLOBAL_HELP: &'static str = "A service gets the browser by injection: the window controls from `@Needs windowControls`, and each other browser API from its own `@Needs` field. Keep state that many services share in a service that they inject (ADR 0087).";
    const FETCH_HELP: &'static str = "Move the request into a method of an HTTPClient, such as `@GetRequest(\"/api/products\")`, and inject the client with `@Needs`. Mark a public read `Cached`, so the service worker may answer it (ADR 0087).";

    pub(crate) fn diagnostics<'a>(
        files: impl Iterator<Item = (&'a Path, &'a [u8])>,
    ) -> Vec<Diagnostic> {
        SafetyUtils::each_file(files, Self::file_diagnostics)
    }

    fn file_diagnostics(path: &Path, bytes: &[u8]) -> Vec<Diagnostic> {
        let portable = SafetyUtils::portable(path);
        let extension = Self::extension(path);
        let service = [
            portable.starts_with(Self::SERVICES),
            Self::SCRIPTS.contains(&extension),
        ] == [true, true];
        if !service {
            return Vec::new();
        }
        let code = CodeText::of(extension, CodeText::source(path, bytes));
        let global = Self::global_use(&code).map(|at| Self::global_refusal(&portable, &code, at));
        let fetch = Self::fetch_call(&code).map(|at| Self::fetch_refusal(&portable, &code, at));
        global.into_iter().chain(fetch).collect()
    }

    fn extension(path: &Path) -> &str {
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("")
    }

    fn global_use(code: &str) -> Option<usize> {
        code.match_indices(Self::GLOBAL)
            .map(|(at, _)| at)
            .find(|at| Self::is_word(code, *at, Self::GLOBAL))
    }

    fn fetch_call(code: &str) -> Option<usize> {
        code.match_indices(Self::FETCH)
            .map(|(at, _)| at)
            .find(|at| Self::is_fetch_call(code, *at))
    }

    fn is_fetch_call(code: &str, at: usize) -> bool {
        let before = &code[..at];
        let owned = Self::FETCH_OWNERS.iter().any(|owner| {
            before.ends_with(owner) && Self::starts_word(before, before.len() - owner.len())
        });
        let bare = Self::is_word(code, at, Self::FETCH);
        let opens = code[at + Self::FETCH.len()..].trim_start().starts_with('(');
        let ends = Self::ends_word(&code[at + Self::FETCH.len()..]);
        [[bare, owned].contains(&true), opens, ends] == [true, true, true]
    }

    fn is_word(code: &str, at: usize, word: &str) -> bool {
        let ends = Self::ends_word(&code[at + word.len()..]);
        let after_dot = code[..at].ends_with('.');
        [Self::starts_word(code, at), ends, !after_dot] == [true, true, true]
    }

    fn starts_word(code: &str, at: usize) -> bool {
        !code[..at]
            .chars()
            .next_back()
            .is_some_and(Self::continues_name)
    }

    fn ends_word(rest: &str) -> bool {
        !rest.chars().next().is_some_and(Self::continues_name)
    }

    fn continues_name(character: char) -> bool {
        character.is_alphanumeric() || character == '_' || character == '$'
    }

    fn global_refusal(portable: &str, code: &str, at: usize) -> Diagnostic {
        let message = format!(
            "'{portable}' line {} reads globalThis. A Tac service never reads the global object.",
            SafetyUtils::line_at(code, at)
        );
        SafetyUtils::refusal(
            Self::GLOBAL_CODE,
            message,
            Self::GLOBAL_HELP,
            (portable, at),
        )
    }

    fn fetch_refusal(portable: &str, code: &str, at: usize) -> Diagnostic {
        let message = format!(
            "'{portable}' line {} calls fetch. A Tac service reaches the network only through an HTTPClient.",
            SafetyUtils::line_at(code, at)
        );
        SafetyUtils::refusal(Self::FETCH_CODE, message, Self::FETCH_HELP, (portable, at))
    }
}
