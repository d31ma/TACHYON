use super::YonConvention;
use super::native_http::NativeHttp;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct HandlerCheck;

impl HandlerCheck {
    const RETIRED_DECORATORS: [&str; 1] = ["Relay"];
    const TYPE_MODIFIERS: [&str; 15] = [
        "public",
        "private",
        "internal",
        "protected",
        "final",
        "open",
        "abstract",
        "sealed",
        "static",
        "pub",
        "pub(crate)",
        "export",
        "default",
        "data",
        "partial",
    ];
    const TYPE_KEYWORDS: [&str; 5] = ["class", "struct", "object", "interface", "record"];
    const RETIRED_HELP: &str = "Export or declare a plain type named Handler. Put HTTP methods directly on it, call relay(...) explicitly, and use a generator method to stream.";
    const PLAIN_HELP: &str = "Declare a plain class, object, or struct named Handler with HTTP methods, or use an annotated Controller with HTTP verb-named instance methods (docs/YON_ANNOTATIONS.md).";
    const GO_HELP: &str = "Declare a plain type named Handler with HTTP methods, for example `type Handler struct{}` and `func (Handler) GET(request *http.Request) *http.Response`.";

    pub(crate) fn check(source: &Path, contents: &str) -> Result<(), Failure> {
        NativeHttp::check(source, contents)?;
        let events = crate::events::extract(source, contents, "yon")?;
        let contents = events
            .as_ref()
            .map_or(contents, |module| module.source.as_str());
        let prepared = crate::yon::prepare_required_args(source, contents)?;
        let contents = prepared.as_deref().unwrap_or(contents);
        let is_test = crate::behavior::test_configuration_name(source, contents, "yon")?.is_some();
        let behavior_source = crate::behavior::lower(source, contents, "yon")?;
        let contents = behavior_source.as_deref().unwrap_or(contents);
        let parsed = Self::parses(source, contents, is_test)?;
        Self::check_unparsed(source, contents, parsed)
    }

    fn parses(source: &Path, contents: &str, is_test: bool) -> Result<bool, Failure> {
        let parsed = if is_test {
            crate::yon::parser::parse_test(source, contents)?
        } else {
            crate::yon::parser::parse(source, contents)?
        };
        Ok(parsed.is_some())
    }

    fn check_unparsed(source: &Path, contents: &str, parsed: bool) -> Result<(), Failure> {
        if parsed {
            return Ok(());
        }
        Self::check_plain(source, contents)
    }

    fn check_plain(source: &Path, contents: &str) -> Result<(), Failure> {
        let portable = source.to_string_lossy().replace('\\', "/");
        let executable = YonConvention::executable_source(source, contents)
            .map_err(|message| Self::unreadable_source_failure(&portable, contents, message))?;
        let problem = Self::handler_problem(source, &portable, &executable);
        problem.map_or(Ok(()), |(message, help)| {
            Err(Self::handler_failure(&portable, contents, message, help))
        })
    }

    fn handler_problem(
        source: &Path,
        portable: &str,
        executable: &str,
    ) -> Option<(String, &'static str)> {
        if YonConvention::extension(source) == "go" {
            return Self::go_problem(portable, executable);
        }
        Self::plain_problem(portable, executable)
    }

    fn go_problem(portable: &str, executable: &str) -> Option<(String, &'static str)> {
        let declared = Self::declares_go_handler_type(executable);
        (!declared).then(|| Self::missing_handler(portable, Self::GO_HELP))
    }

    fn plain_problem(portable: &str, executable: &str) -> Option<(String, &'static str)> {
        Self::retired_problem(portable, executable)
            .or_else(|| Self::missing_handler_problem(portable, executable))
    }

    fn retired_problem(portable: &str, executable: &str) -> Option<(String, &'static str)> {
        Self::uses_retired_decorator(executable).then(|| {
            (
                format!("'{portable}' uses a retired Yon decorator."),
                Self::RETIRED_HELP,
            )
        })
    }

    fn missing_handler_problem(portable: &str, executable: &str) -> Option<(String, &'static str)> {
        let declared = Self::declares_handler_type(executable);
        (!declared).then(|| Self::missing_handler(portable, Self::PLAIN_HELP))
    }

    fn missing_handler(portable: &str, help: &'static str) -> (String, &'static str) {
        (
            format!("'{portable}' does not declare a Yon Handler."),
            help,
        )
    }

    fn declares_go_handler_type(executable: &str) -> bool {
        executable
            .lines()
            .any(|line| Self::go_type_name(line) == Some("Handler"))
    }

    fn go_type_name(line: &str) -> Option<&str> {
        let rest = YonConvention::strip_leading_word(line.trim(), &["type"])?;
        rest.split(|character| !YonConvention::identifier_character(character))
            .next()
    }

    fn uses_retired_decorator(executable: &str) -> bool {
        Self::RETIRED_DECORATORS
            .iter()
            .any(|name| Self::uses_decorator(executable, name))
    }

    fn uses_decorator(executable: &str, name: &str) -> bool {
        let markers = [format!("@{name}"), format!("[{name}"), format!("#[{name}")];
        markers.iter().any(|marker| executable.contains(marker))
    }

    fn declares_handler_type(executable: &str) -> bool {
        executable
            .lines()
            .any(|line| Self::type_name(line).as_deref() == Some("Handler"))
    }

    fn type_name(line: &str) -> Option<String> {
        let rest = Self::skip_modifiers(line.trim());
        let after_keyword = YonConvention::strip_leading_word(rest, &Self::TYPE_KEYWORDS)?;
        let name: String = after_keyword
            .chars()
            .take_while(|character| YonConvention::identifier_character(*character))
            .collect();
        (!name.is_empty()).then_some(name)
    }

    fn skip_modifiers(line: &str) -> &str {
        let mut rest = line;
        while let Some(after_modifier) =
            YonConvention::strip_leading_word(rest, &Self::TYPE_MODIFIERS)
        {
            rest = after_modifier;
        }
        rest
    }

    fn unreadable_source_failure(portable: &str, contents: &str, message: &str) -> Failure {
        Failure::one(diagnostic(
            2004,
            format!("Cannot read Yon source '{portable}': {message}."),
            Some(String::from(
                "Close the string or comment before running the handler.",
            )),
            source_span(portable, 0, contents.len()),
        ))
    }

    fn handler_failure(portable: &str, contents: &str, message: String, help: &str) -> Failure {
        Failure::one(diagnostic(
            2015,
            message,
            Some(String::from(help)),
            source_span(portable, 0, contents.len()),
        ))
    }
}
