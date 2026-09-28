use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct BehaviorsImport;

impl BehaviorsImport {
    pub(crate) const SPECIFIER: &str = "tachyon:behaviors";
    const HELP: &str = "Tachyon annotations, lifecycle names, Behaviors and the framework types are global in every language (ADR 0046); see docs/BEHAVIOR_ANNOTATIONS.md.";

    pub(crate) fn refuse(path: &Path, source: &str) -> Result<(), Failure> {
        match Self::specifier_positions(path, source).first() {
            Some(at) => Err(Self::failure(path, source, *at)),
            None => Ok(()),
        }
    }

    pub(crate) fn specifier_positions(path: &Path, source: &str) -> Vec<usize> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !matches!(extension, "js" | "ts") || !source.contains(Self::SPECIFIER) {
            return Vec::new();
        }
        let mask = crate::lexical::code_mask(extension, source);
        let mut positions = Vec::new();
        for (at, _) in source.match_indices(Self::SPECIFIER) {
            if Self::is_import_specifier(source, &mask.code, at) {
                positions.push(at);
            }
        }
        positions
    }

    fn is_import_specifier(source: &str, code: &[bool], at: usize) -> bool {
        let bytes = source.as_bytes();
        let Some(quote_at) = at.checked_sub(1) else {
            return false;
        };
        let quote = bytes[quote_at];
        if !matches!(quote, b'\'' | b'"' | b'`') || code[quote_at] {
            return false;
        }
        if bytes.get(at + Self::SPECIFIER.len()) != Some(&quote) {
            return false;
        }
        matches!(
            Self::previous_code_word(bytes, code, quote_at),
            "from" | "import"
        )
    }

    fn previous_code_word<'a>(bytes: &'a [u8], code: &[bool], end: usize) -> &'a str {
        let mut word_end = end;
        while word_end > 0 && Self::is_skipped(bytes, code, word_end - 1) {
            word_end -= 1;
        }
        let mut word_start = word_end;
        while word_start > 0 && Self::is_word_byte(bytes, code, word_start - 1) {
            word_start -= 1;
        }
        std::str::from_utf8(&bytes[word_start..word_end]).unwrap_or("")
    }

    fn is_skipped(bytes: &[u8], code: &[bool], at: usize) -> bool {
        !code[at] || bytes[at].is_ascii_whitespace() || bytes[at] == b'('
    }

    fn is_word_byte(bytes: &[u8], code: &[bool], at: usize) -> bool {
        code[at] && (bytes[at].is_ascii_alphanumeric() || matches!(bytes[at], b'_' | b'$'))
    }

    fn failure(path: &Path, source: &str, at: usize) -> Failure {
        let file = path.to_string_lossy();
        let line = source[..at].matches('\n').count() + 1;
        Failure::one(diagnostic(
            1308,
            format!(
                "{file}:{line}: Tachyon annotations and types are global; delete this import from '{}'.",
                Self::SPECIFIER
            ),
            Some(Self::HELP.to_owned()),
            source_span(&file, at, at + Self::SPECIFIER.len()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::BehaviorsImport;
    use std::path::Path;

    fn refused(path: &str, source: &str) -> String {
        match BehaviorsImport::refuse(Path::new(path), source) {
            Ok(()) => panic!("expected a refusal for {source}"),
            Err(failure) => failure.to_string(),
        }
    }

    fn accepted(path: &str, source: &str) {
        if let Err(failure) = BehaviorsImport::refuse(Path::new(path), source) {
            panic!("expected {source} to be accepted: {failure}");
        }
    }

    #[test]
    fn every_import_form_is_refused_with_the_file_and_line() {
        for source in [
            "import { Service } from 'tachyon:behaviors';",
            "import type { BehaviorContext } from \"tachyon:behaviors\"",
            "import * as Behavior from 'tachyon:behaviors';",
            "import Default from 'tachyon:behaviors';",
            "import 'tachyon:behaviors';",
            "const behaviors = await import('tachyon:behaviors');",
            "export { Service } from 'tachyon:behaviors';",
            "import { Service } from /* old */ 'tachyon:behaviors';",
        ] {
            let message = refused("client/services/CartService.ts", &format!("\n{source}"));
            assert!(message.contains("TY1308"), "{message}");
            assert!(
                message.contains("client/services/CartService.ts:2: Tachyon annotations and types are global; delete this import"),
                "{message}"
            );
        }
    }

    #[test]
    fn a_split_import_is_refused_at_the_line_of_its_module_name() {
        let message = refused(
            "server/controllers/CartController.js",
            "import { Service }\nfrom 'tachyon:behaviors'\n",
        );
        assert!(
            message.contains("server/controllers/CartController.js:2: Tachyon annotations"),
            "{message}"
        );
    }

    #[test]
    fn mentions_outside_an_import_are_accepted() {
        for source in [
            "// import { Service } from 'tachyon:behaviors';\nclass Plain {}",
            "/* from 'tachyon:behaviors' */\nclass Plain {}",
            "const text = \"import { Service } from 'tachyon:behaviors'\";",
            "const name = 'tachyon:behaviors';",
            "import { helper } from '/shared/scripts/tachyon:behaviors-helper.js';",
        ] {
            accepted("client/services/CartService.js", source);
        }
    }

    #[test]
    fn other_languages_are_not_checked() {
        accepted(
            "server/controllers/CartController.py",
            "from 'tachyon:behaviors'",
        );
    }
}
