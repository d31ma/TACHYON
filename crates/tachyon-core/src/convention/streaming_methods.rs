use super::YonConvention;
use super::handler_body::HandlerBody;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) struct StreamingMethods;

impl StreamingMethods {
    const STREAMING_EXTENSIONS: [&str; 7] = ["js", "ts", "py", "php", "kt", "cs", "dart"];
    const HTTP_METHODS: [&str; 8] = [
        "GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD", "TRACE",
    ];
    const FUNCTION_KEYWORDS: [&str; 4] = ["fn", "def", "fun", "function"];
    const CONTROL_WORDS: [&str; 12] = [
        "if", "for", "while", "switch", "catch", "else", "do", "try", "using", "lock", "foreach",
        "match",
    ];
    const CALL_SUFFIXES: [&str; 7] = ["::", "->", "?.", ".", "@", "[", "#["];
    const STATEMENT_WORDS: [&str; 6] = ["new", "raise", "throw", "return", "await", "yield"];
    const BODY_MODIFIERS: [&str; 3] = ["async*", "async", "sync*"];
    const BODY_STARTS: [&str; 4] = ["{", "=>", "->", "where"];

    pub(crate) fn find(source: &Path, contents: &str) -> BTreeSet<String> {
        Self::lowered_source(source, contents)
            .map(|lowered| Self::from_lowered(source, &lowered))
            .unwrap_or_default()
    }

    fn lowered_source(source: &Path, contents: &str) -> Option<String> {
        let prepared = crate::yon::prepare_required_args(source, contents).ok()?;
        let contents = prepared.as_deref().unwrap_or(contents);
        let behavior_source = crate::behavior::lower(source, contents, "yon").ok()?;
        Some(behavior_source.unwrap_or_else(|| contents.to_owned()))
    }

    fn from_lowered(source: &Path, contents: &str) -> BTreeSet<String> {
        Self::parsed_streaming(source, contents)
            .unwrap_or_else(|| Self::scanned_streaming(source, contents))
    }

    fn parsed_streaming(source: &Path, contents: &str) -> Option<BTreeSet<String>> {
        let parsed = crate::yon::parser::parse(source, contents).ok().flatten()?;
        let methods = parsed
            .plan
            .routes
            .into_iter()
            .filter(|route| route.streaming)
            .map(|route| route.verb)
            .collect();
        Some(methods)
    }

    fn scanned_streaming(source: &Path, contents: &str) -> BTreeSet<String> {
        Self::streaming_executable(source, contents)
            .map(|executable| Self::handler_methods(&HandlerBody::find(source, &executable)))
            .unwrap_or_default()
    }

    fn streaming_executable(source: &Path, contents: &str) -> Option<String> {
        let extension = YonConvention::extension(source);
        let supported = Self::STREAMING_EXTENSIONS.contains(&extension.as_str());
        supported
            .then(|| YonConvention::executable_source(source, contents).ok())
            .flatten()
    }

    fn handler_methods(handler: &str) -> BTreeSet<String> {
        Self::declared_methods(handler)
            .into_iter()
            .map(|method| method.to_ascii_uppercase())
            .filter(|method| Self::HTTP_METHODS.contains(&method.as_str()))
            .filter(|method| Self::yields(handler, method))
            .collect()
    }

    fn yields(contents: &str, method: &str) -> bool {
        contents
            .lines()
            .skip_while(|line| !Self::names_method(&Self::declared_methods(line), method))
            .find_map(|line| Self::line_answer(line, method))
            .unwrap_or(false)
    }

    fn line_answer(line: &str, method: &str) -> Option<bool> {
        let declarations = Self::declared_methods(line);
        let own = Self::names_method(&declarations, method);
        let yields = Self::has_word(line, "yield");
        match (own, declarations.is_empty(), yields) {
            (false, false, _) => Some(false),
            (_, _, true) => Some(true),
            _ => None,
        }
    }

    fn names_method(declarations: &[String], method: &str) -> bool {
        declarations
            .iter()
            .any(|name| name.eq_ignore_ascii_case(method))
    }

    fn has_word(contents: &str, word: &str) -> bool {
        contents
            .match_indices(word)
            .any(|(start, _)| YonConvention::is_whole_word(contents, start, start + word.len()))
    }

    fn declared_methods(contents: &str) -> Vec<String> {
        contents.lines().filter_map(Self::declared_method).collect()
    }

    fn declared_method(line: &str) -> Option<String> {
        let line = line.trim();
        let open = line.find('(')?;
        let before = &line[..open];
        let name_start = before
            .trim_end_matches(YonConvention::identifier_character)
            .len();
        let name = Self::method_name(&before[name_start..])?;
        let head = before[..name_start].trim_end();
        Self::is_declaration(line, open, head).then(|| name.to_owned())
    }

    fn method_name(name: &str) -> Option<&str> {
        let usable = !name.is_empty() && !Self::CONTROL_WORDS.contains(&name);
        usable.then_some(name)
    }

    fn is_declaration(line: &str, open: usize, head: &str) -> bool {
        let last_word = head.split_whitespace().next_back();
        let checks = [
            Self::follows_function_keyword(last_word),
            Self::has_declared_body(line, open, head, last_word),
        ];
        checks.contains(&true)
    }

    fn follows_function_keyword(last_word: Option<&str>) -> bool {
        last_word.is_some_and(|word| Self::FUNCTION_KEYWORDS.contains(&word))
    }

    fn has_declared_body(line: &str, open: usize, head: &str, last_word: Option<&str>) -> bool {
        let checks = [
            !head.is_empty(),
            !Self::is_call(head),
            !Self::is_statement(last_word),
            Self::body_follows(line, open),
        ];
        checks.iter().all(|passed| *passed)
    }

    fn is_call(head: &str) -> bool {
        Self::CALL_SUFFIXES
            .iter()
            .any(|suffix| head.ends_with(suffix))
    }

    fn is_statement(last_word: Option<&str>) -> bool {
        last_word.is_some_and(|word| Self::STATEMENT_WORDS.contains(&word))
    }

    fn body_follows(line: &str, open: usize) -> bool {
        let pairs = line.char_indices().skip(open);
        YonConvention::closing_index(pairs, '(', ')')
            .is_some_and(|close| Self::declares_body(line[close + 1..].trim_start()))
    }

    fn declares_body(after_parameters: &str) -> bool {
        let mut candidates = vec![after_parameters];
        candidates.extend(
            Self::BODY_MODIFIERS
                .iter()
                .filter_map(|modifier| Self::after_modifier(after_parameters, modifier)),
        );
        candidates.into_iter().any(Self::opens_body)
    }

    fn after_modifier<'a>(after_parameters: &'a str, modifier: &str) -> Option<&'a str> {
        let spaced = after_parameters
            .strip_prefix(modifier)?
            .strip_prefix(char::is_whitespace)?;
        Some(spaced.trim_start())
    }

    fn opens_body(rest: &str) -> bool {
        let starts_body = Self::BODY_STARTS
            .iter()
            .any(|start| rest.starts_with(start));
        rest.is_empty() || starts_body
    }
}
