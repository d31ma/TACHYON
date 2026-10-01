#![allow(clippy::needless_bitwise_bool)]

use super::super::super::Input;
use super::super::Tokens;
use super::super::cohesion::{CohesionCheck, Spelling, Words};
use super::form_pattern::FormPattern;
use super::receiver_scope::ReceiverScope;
use super::simpler_form::SimplerForm;
use std::path::Path;

pub(super) struct ReceiverForms;

impl ReceiverForms {
    const EXTENSIONS: [&'static str; 6] = ["java", "kt", "cs", "groovy", "swift", "dart"];
    const MAX_SOURCE_BYTES: usize = 256 * 1024;
    const MAX_WORDS: usize = 16 * 1024;
    const MAX_ACCESSES: usize = 256;
    const MINIFIED: &'static str = ".min.";
    const SWIFT: &'static str = "swift";
    const RECEIVERS: [&'static str; 2] = ["this", "self"];
    const RECEIVER_PREFIXES: [&'static str; 2] = ["this.", "self."];

    pub(super) fn supports(extension: &str) -> bool {
        Self::EXTENSIONS.contains(&extension)
    }

    pub(super) fn find(path: &Path, source: &str) -> Vec<SimplerForm> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let wanted = Self::supports(extension) & Self::readable(path, source);
        wanted
            .then(|| Self::scan(path, source, extension))
            .flatten()
            .unwrap_or_default()
    }

    fn readable(path: &Path, source: &str) -> bool {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        (source.len() <= Self::MAX_SOURCE_BYTES) & !name.contains(Self::MINIFIED)
    }

    fn scan(path: &Path, source: &str, extension: &str) -> Option<Vec<SimplerForm>> {
        let input = Input::new(path, source);
        let tokens = Tokens::read(&input).ok()?;
        tokens.check_complete(&input).ok()?;
        let words = Words::read(&tokens, source);
        let spelling = Spelling::of(crate::groovy_source::GroovySource::syntax(extension))?;
        let receiver = usize::from(extension == Self::SWIFT);
        let accesses = Self::accesses(&words, Self::RECEIVERS[receiver]);
        Self::bounded(&words, &accesses).then(|| Self::forms(&words, &spelling, source, &accesses))
    }

    fn accesses(words: &Words<'_>, receiver: &str) -> Vec<usize> {
        (0..words.len())
            .filter(|&at| ReceiverScope::is_access(words, at, receiver))
            .collect()
    }

    fn bounded(words: &Words<'_>, accesses: &[usize]) -> bool {
        (words.len() <= Self::MAX_WORDS) & (accesses.len() <= Self::MAX_ACCESSES)
    }

    fn forms(
        words: &Words<'_>,
        spelling: &Spelling,
        source: &str,
        accesses: &[usize],
    ) -> Vec<SimplerForm> {
        let methods = CohesionCheck::methods(words, spelling);
        methods
            .iter()
            .flat_map(|method| {
                let scope = ReceiverScope::new(words, &methods, method);
                accesses
                    .iter()
                    .filter(|&&at| scope.permits(at))
                    .filter_map(|&at| Self::form(words, source, at))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn form(words: &Words<'_>, source: &str, at: usize) -> Option<SimplerForm> {
        let start = words.start(at);
        let end = words.start(at + 2);
        let plain = source
            .get(start..end)
            .is_some_and(|prefix| Self::RECEIVER_PREFIXES.contains(&prefix));
        plain.then(|| SimplerForm::new(FormPattern::UnnecessaryReceiver, start, end, String::new()))
    }
}
