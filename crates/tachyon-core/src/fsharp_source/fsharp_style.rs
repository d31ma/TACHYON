use super::{FsharpLexer, FsharpSource, SourceLine};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct FsharpStyle;

impl FsharpStyle {
    const CODE: u16 = 1335;
    const FILE_LEVEL_WORDS: [&'static str; 4] = ["module ", "namespace ", "let ", "do "];
    const FILE_LEVEL: &'static str = "An F# file holds its type and nothing else at file level: Tachyon places every file in one namespace, and a file-level `let` or `do` is a file-level variable. Remove the `module` or `namespace` line and move the value into the type";
    const INLINE_ATTRIBUTE: &'static str =
        "Place each F# attribute on its own line, above the type or member it marks";
    const HELP: &'static str = "See ADR 0073 for the F# forms Tachyon reads.";

    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        let wanted = FsharpSource::is_fsharp(path);
        let problem = wanted.then(|| Self::first_problem(source)).flatten();
        problem.map_or(Ok(()), |(at, message)| {
            Err(Self::failure(path, at, message))
        })
    }

    fn first_problem(source: &str) -> Option<(usize, &'static str)> {
        let plain = FsharpLexer::without_comments(source);
        let code = FsharpLexer::code_mask(&plain).code;
        let mut start = 0;
        let mut found = None;
        for line in SourceLine::split(&plain, &code) {
            found = found.or_else(|| Self::line_problem(line.text).map(|message| (start, message)));
            start += line.text.len() + 1;
        }
        found
    }

    fn line_problem(text: &str) -> Option<&'static str> {
        let trimmed = text.trim();
        if Self::FILE_LEVEL_WORDS
            .iter()
            .any(|word| text.starts_with(word))
        {
            Some(Self::FILE_LEVEL)
        } else if trimmed.starts_with("[<") && !Self::after_attributes(trimmed).is_empty() {
            Some(Self::INLINE_ATTRIBUTE)
        } else {
            None
        }
    }

    fn after_attributes(trimmed: &str) -> &str {
        let mut rest = trimmed;
        while let Some(next) = rest
            .strip_prefix("[<")
            .and_then(|inside| inside.split_once(">]"))
        {
            rest = next.1.trim_start();
        }
        rest
    }

    fn failure(path: &Path, at: usize, message: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(String::from(Self::HELP)),
            source_span(&portable, at, at + 1),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpStyle;
    use std::path::Path;

    fn problem(source: &str) -> Option<String> {
        FsharpStyle::check(Path::new("server/services/OrderService.fs"), source)
            .err()
            .map(|failure| failure.to_string())
    }

    #[test]
    fn module_lines_file_level_values_and_inline_attributes_are_refused() {
        assert!(
            problem("module Orders\n\ntype A() = class end\n")
                .is_some_and(|text| text.contains("TY1335"))
        );
        assert!(problem("let limit = 3\ntype A() = class end\n").is_some());
        assert!(problem("[<Service>] type OrderService() = class end\n").is_some());
    }

    #[test]
    fn a_plain_type_file_and_other_languages_pass() {
        let source = "open Models\n\n[<Service>]\ntype OrderService() =\n    let mutable count = 0\n    member this.Add([<FromBody>] order: Order) : int = count\n";
        assert!(problem(source).is_none());
        assert!(FsharpStyle::check(Path::new("server/services/A.cs"), "let x = 1").is_ok());
    }
}
