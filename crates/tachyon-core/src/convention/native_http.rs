use super::YonConvention;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct NativeHttp;

impl NativeHttp {
    const REMOVED: [&str; 4] = ["YonRequest", "YonResponse", "YonJSON", "YonEmpty"];
    const LANGUAGES_THAT_HAD_THEM: [&str; 7] = ["php", "java", "kt", "cs", "rs", "dart", "go"];
    const MIGRATION: &str = "`ty migrate fix --write` changes the simple cases (docs/adr/0062-yon-uses-native-http-types.md).";

    pub(crate) fn request(extension: &str) -> &'static str {
        Self::request_type(extension)
    }

    pub(crate) fn response(extension: &str) -> &'static str {
        Self::response_type(extension)
    }

    pub(super) fn check(source: &Path, contents: &str) -> Result<(), Failure> {
        Self::checked_source(source, contents)
            .map_or(Ok(()), |executable| Self::check_names(source, &executable))
    }

    fn checked_source(source: &Path, contents: &str) -> Option<String> {
        let extension = YonConvention::extension(source);
        let affected = Self::LANGUAGES_THAT_HAD_THEM.contains(&extension.as_str());
        affected
            .then(|| YonConvention::executable_source(source, contents).ok())
            .flatten()
    }

    fn check_names(source: &Path, executable: &str) -> Result<(), Failure> {
        let found = Self::first_removed_name(executable);
        found.map_or(Ok(()), |(name, start)| {
            Err(Self::removed_failure(source, name, start))
        })
    }

    fn first_removed_name(executable: &str) -> Option<(&'static str, usize)> {
        Self::REMOVED
            .iter()
            .find_map(|name| Self::first_use(executable, name))
    }

    fn first_use(executable: &str, name: &'static str) -> Option<(&'static str, usize)> {
        executable
            .match_indices(name)
            .map(|(start, _)| start)
            .find(|start| YonConvention::is_whole_word(executable, *start, start + name.len()))
            .map(|start| (name, start))
    }

    fn removed_failure(source: &Path, name: &str, start: usize) -> Failure {
        let portable = source.to_string_lossy().replace('\\', "/");
        let end = start + name.len();
        Failure::one(diagnostic(
            2015,
            format!("'{portable}' uses {name}, which Tachyon removed."),
            Some(Self::advice(&YonConvention::extension(source), name)),
            source_span(&portable, start, end),
        ))
    }

    fn advice(extension: &str, name: &str) -> String {
        let request = Self::request_type(extension);
        let response = Self::response_type(extension);
        let sentence = Self::advice_sentence(name, request, response);
        format!("{sentence} {}", Self::MIGRATION)
    }

    fn advice_sentence(name: &str, request: &str, response: &str) -> String {
        if name == "YonRequest" {
            return format!("A handler receives the native request: {request}.");
        }
        format!("A handler answers with the native response: {response}.")
    }

    fn request_type(extension: &str) -> &'static str {
        match extension {
            "js" | "ts" => {
                "the Fetch Request (request.method, request.url, request.headers.get(name), await request.text())"
            }
            "py" => {
                "urllib.request.Request (request.get_method(), request.full_url, request.get_header(name), request.data)"
            }
            "rb" => {
                "a Net::HTTP request (request.method, request.path, request[name], request.body)"
            }
            "php" => "the superglobals $_SERVER, $_GET, $_POST, $_COOKIE and php://input",
            "java" | "groovy" | "kt" => "com.sun.net.httpserver.HttpExchange",
            "cs" => "System.Net.Http.HttpRequestMessage",
            "rs" => "http::Request<Vec<u8>> from the http crate",
            "dart" => "the Request of package:shelf",
            "go" => "*http.Request from net/http",
            _ => "the native request of its language",
        }
    }

    fn response_type(extension: &str) -> &'static str {
        match extension {
            "js" | "ts" => {
                "a Fetch Response (new Response(body, { status, headers }) or Response.json(value))"
            }
            "py" => "a (HTTPStatus, headers, body) tuple",
            "rb" => "a [status, headers, body] array",
            "php" => "http_response_code(), header() and echo",
            "java" | "groovy" | "kt" => {
                "the same HttpExchange: getResponseHeaders(), sendResponseHeaders() and getResponseBody()"
            }
            "cs" => "System.Net.Http.HttpResponseMessage",
            "rs" => "http::Response<Vec<u8>> from the http crate",
            "dart" => "the Response of package:shelf",
            "go" => "*http.Response from net/http",
            _ => "the native response of its language",
        }
    }
}
