use super::{CodeText, QueryArgument, QuerySource as Source, SafetyUtils};
use std::collections::BTreeSet;
use std::ops::Range;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct QueryCheck;

impl QueryCheck {
    const CODE: u16 = 1325;
    const TYPE_NAME: &'static str = "DataSource";
    const SEPARATORS: [&'static str; 3] = [".", "->", "::"];
    const METHODS: &'static str = "query Query queryRow QueryRow QueryContext execute Execute executeQuery executeUpdate exec Exec ExecContext prepare Prepare prepareStatement ExecuteSqlRaw";
    const DATA_WORDS: &'static str =
        "datasource database db cursor connection conn statement stmt pool";
    const HELP: &'static str = "Write the query text as one text literal or an UPPER_CASE constant, and pass every value in the parameter list, for example query(\"SELECT id FROM products WHERE name = $1\", [name]). A value joined into the query text lets a caller change the query (SQL injection). Redis keys and values are always parameters (ADR 0068).";

    pub(crate) fn diagnostics<'a>(
        files: impl Iterator<Item = (&'a Path, &'a [u8])>,
    ) -> Vec<Diagnostic> {
        Self::sources(files)
            .iter()
            .flat_map(Self::file_diagnostics)
            .collect()
    }

    fn sources<'a>(files: impl Iterator<Item = (&'a Path, &'a [u8])>) -> Vec<Source<'a>> {
        files
            .filter_map(|(path, bytes)| Self::source(path, bytes))
            .collect()
    }

    fn source<'a>(path: &'a Path, bytes: &'a [u8]) -> Option<Source<'a>> {
        let extension = path.extension()?.to_str()?;
        let text = CodeText::source(path, bytes);
        let code = CodeText::of(extension, text);
        Some(Source {
            path,
            extension,
            text,
            code,
        })
    }

    fn file_diagnostics(source: &Source<'_>) -> Vec<Diagnostic> {
        let receivers = Self::data_source_names(&source.code);
        let refused = Self::refused(source, Self::calls(&source.code, &receivers));
        refused
            .iter()
            .map(|argument| Self::diagnostic(source, argument.start))
            .collect()
    }

    fn refused(source: &Source<'_>, arguments: Vec<Range<usize>>) -> Vec<Range<usize>> {
        arguments
            .into_iter()
            .filter(|argument| !Self::safe_argument(source, argument))
            .collect()
    }

    fn data_source_names(code: &str) -> BTreeSet<String> {
        Self::typed_lines(code)
            .into_iter()
            .flat_map(Self::words)
            .collect()
    }

    fn typed_lines(code: &str) -> Vec<&str> {
        code.lines()
            .filter(|line| line.contains(Self::TYPE_NAME))
            .collect()
    }

    fn words(line: &str) -> Vec<String> {
        line.split(|character: char| !Self::name_character(character))
            .filter(|word| !word.is_empty())
            .map(Self::plain)
            .collect()
    }

    fn calls(code: &str, receivers: &BTreeSet<String>) -> Vec<Range<usize>> {
        Self::METHODS
            .split(' ')
            .flat_map(|method| Self::method_calls(code, method, receivers))
            .collect()
    }

    fn method_calls(code: &str, method: &str, receivers: &BTreeSet<String>) -> Vec<Range<usize>> {
        let opening = format!("{method}(");
        code.match_indices(&opening)
            .filter_map(|(at, _)| Self::guarded_argument(code, at, method, receivers))
            .collect()
    }

    fn guarded_argument(
        code: &str,
        at: usize,
        method: &str,
        receivers: &BTreeSet<String>,
    ) -> Option<Range<usize>> {
        let receiver = Self::receiver(&code[..at])?;
        Self::is_data_access(&receiver, receivers).then_some(())?;
        Self::first_argument(code, at + method.len() + 1)
    }

    fn receiver(before: &str) -> Option<String> {
        let separator = Self::separator_before(before)?;
        let owner = &before[..before.len() - separator.len()];
        Some(Self::plain(Self::last_name(owner)))
    }

    fn separator_before(before: &str) -> Option<&'static str> {
        Self::SEPARATORS
            .into_iter()
            .find(|separator| before.ends_with(separator))
    }

    fn last_name(owner: &str) -> &str {
        let start = owner
            .rfind(|character: char| !Self::name_character(character))
            .map_or(0, |at| at + 1);
        &owner[start..]
    }

    fn is_data_access(receiver: &str, receivers: &BTreeSet<String>) -> bool {
        let named = Self::DATA_WORDS
            .split(' ')
            .any(|word| Self::names_data(receiver, word));
        [named, receivers.contains(receiver)].contains(&true)
    }

    fn names_data(receiver: &str, word: &str) -> bool {
        [receiver.starts_with(word), receiver.ends_with(word)].contains(&true)
    }

    fn first_argument(code: &str, start: usize) -> Option<Range<usize>> {
        let end = Self::argument_end(&code.as_bytes()[start..])?;
        Some(start..start + end)
    }

    fn argument_end(bytes: &[u8]) -> Option<usize> {
        let mut depth = 0_usize;
        bytes.iter().position(|byte| Self::step(&mut depth, *byte))
    }

    fn step(depth: &mut usize, byte: u8) -> bool {
        let ends = Self::ends_argument(*depth, byte);
        *depth = Self::next_depth(*depth, byte);
        ends
    }

    fn next_depth(depth: usize, byte: u8) -> usize {
        match byte {
            b'(' | b'[' | b'{' => depth + 1,
            b')' | b']' | b'}' => depth.saturating_sub(1),
            _ => depth,
        }
    }

    fn ends_argument(depth: usize, byte: u8) -> bool {
        [depth == 0, matches!(byte, b',' | b')')] == [true, true]
    }

    fn safe_argument(source: &Source<'_>, argument: &Range<usize>) -> bool {
        let text = source.text.get(argument.clone()).unwrap_or("");
        let code = source.code.get(argument.clone()).unwrap_or("");
        [
            text.trim().is_empty(),
            QueryArgument::is_safe(source.extension, text, code),
        ]
        .contains(&true)
    }

    fn name_character(character: char) -> bool {
        matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$')
    }

    fn plain(name: &str) -> String {
        name.replace(['_', '$'], "").to_ascii_lowercase()
    }

    fn diagnostic(source: &Source<'_>, at: usize) -> Diagnostic {
        let portable = SafetyUtils::portable(source.path);
        let message = format!(
            "'{portable}' line {} builds query text from pieces instead of passing the values as parameters.",
            SafetyUtils::line_at(&source.code, at)
        );
        SafetyUtils::refusal(Self::CODE, message, Self::HELP, (&portable, at))
    }
}
