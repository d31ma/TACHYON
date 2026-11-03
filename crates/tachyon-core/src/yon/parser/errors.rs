use super::routes::{ReturnType, RouteMarker};
use super::{
    Annotation, Declaration, Input, ManagedType, method_body, method_owner, method_signature,
};
use crate::Failure;
use crate::yon::plan::{ErrorMethod, Role};
use std::collections::BTreeSet;

pub(super) fn is_method(declaration: &Declaration) -> bool {
    declaration
        .annotations
        .iter()
        .any(|marker| marker.name == "handleserror")
}

pub(super) fn read(
    input: &Input<'_>,
    declarations: &[Declaration],
    types: &[ManagedType],
) -> Result<Vec<ErrorMethod>, Failure> {
    let mut handled = BTreeSet::new();
    let methods = declarations
        .iter()
        .filter(|declaration| is_method(declaration))
        .map(|declaration| ErrorRules::read_unique_method(input, declaration, types, &mut handled))
        .collect::<Result<Vec<_>, _>>()?;
    types
        .iter()
        .try_for_each(|managed| ErrorRules::require_handler_method(input, managed, &methods))?;
    Ok(methods)
}

struct ErrorRules;

impl ErrorRules {
    const NOT_ERROR_TYPES: [&'static str; 20] = [
        "any",
        "unknown",
        "object",
        "Object",
        "Any",
        "Any?",
        "dynamic",
        "mixed",
        "interface{}",
        "string",
        "String",
        "str",
        "int",
        "bool",
        "boolean",
        "number",
        "i64",
        "u64",
        "void",
        "Unit",
    ];

    const TYPE_NAME_PUNCTUATION: [u8; 4] = *b"_.:\\";

    const RECEIVERS: [(&'static str, &'static str); 2] = [("py", "self"), ("rs", "&self")];

    const DEFAULT_STATUS: u16 = 500;

    const TYPE_NEEDED: &'static str = "@HandlesError needs one error type on its first parameter, such as OrderNotFound error. The type must be a native error class or type. any, Object, unions and plain values are refused";

    const JSDOC_TYPE_NEEDED: &'static str = "@HandlesError needs one error type for its first parameter. Add @param {OrderNotFound} error to the method's JSDoc. The type must be an Error class";

    const YARD_TYPE_NEEDED: &'static str = "#[HandlesError] needs one error type for its first parameter. Add `# @param error [OrderNotFound]` to the method's YARD comment. The type must be a StandardError class";

    const STATIC_METHOD: &'static str =
        "@HandlesError goes on an instance method. Remove static from the method";

    const STREAMING: &'static str = "@HandlesError returns one response and cannot stream";

    fn require(condition: bool, refusal: impl FnOnce() -> Failure) -> Result<(), Failure> {
        condition.then_some(()).ok_or_else(refusal)
    }

    fn read_unique_method(
        input: &Input<'_>,
        declaration: &Declaration,
        types: &[ManagedType],
        handled: &mut BTreeSet<String>,
    ) -> Result<ErrorMethod, Failure> {
        let method = Self::read_method(input, declaration, types)?;
        let first_handler = handled.insert(Self::handled_key(input.extension, &method.error_type));
        Self::require(first_handler, || {
            input.failure(declaration.start, format!(
                "Two @HandlesError methods handle {}. Each error type needs one handler in the project",
                method.error_type
            ))
        })?;
        Ok(method)
    }

    fn handled_key(extension: &str, error_type: &str) -> String {
        if extension == "php" {
            error_type.to_ascii_lowercase()
        } else {
            error_type.to_owned()
        }
    }

    fn require_handler_method(
        input: &Input<'_>,
        managed: &ManagedType,
        methods: &[ErrorMethod],
    ) -> Result<(), Failure> {
        let lacks_method = managed.provider.role == Role::ErrorHandler
            && !Self::has_method_for(methods, &managed.provider.name);
        Self::require(!lacks_method, || {
            input.failure(
                managed.declaration,
                "An @ErrorHandler class needs at least one @HandlesError instance method",
            )
        })
    }

    fn has_method_for(methods: &[ErrorMethod], owner: &str) -> bool {
        methods.iter().any(|method| method.owner == owner)
    }

    fn read_method(
        input: &Input<'_>,
        declaration: &Declaration,
        types: &[ManagedType],
    ) -> Result<ErrorMethod, Failure> {
        let start = declaration.start;
        let owner = method_owner(input, start, types)?;
        let on_error_handler = types
            .iter()
            .any(|managed| Self::is_error_handler_named(managed, owner));
        Self::require(on_error_handler, || {
            input.failure(
                start,
                "@HandlesError belongs on a method of an @ErrorHandler class",
            )
        })?;
        let status = Self::read_status(input, declaration)?;
        Self::refuse_static(input, start)?;
        let signature = method_signature(input, start)?;
        super::reject_python_static(input, start)?;
        let body = method_body(input, start, signature.end)?;
        Self::require(!signature.generator, || {
            input.failure(start, Self::STREAMING)
        })?;
        Self::require(!super::generator_body(input, &body, start), || {
            input.failure(start, Self::STREAMING)
        })?;
        let list = RouteMarker::parameter_list(input, start)?;
        let documentation = RouteMarker::documentation_before(input, declaration);
        let parameters = Self::read_parameters(input, &list, start)?;
        let php_request = [parameters.len() == 2, input.extension == "php"] == [true, true];
        Self::require(!php_request, || {
            input.failure(start, RouteMarker::PHP_REQUEST_ADVICE)
        })?;
        let error_type = Self::error_type(input, &parameters[0], &documentation, start)?;
        let returns = Self::return_type(input, &list, start, &signature.name, &documentation);
        Ok(ErrorMethod {
            owner: owner.to_owned(),
            method: signature.name,
            error_type,
            takes_request: parameters.len() == 2,
            status,
            returns,
            documentation,
        })
    }

    fn is_error_handler_named(managed: &ManagedType, owner: &str) -> bool {
        managed.provider.name == owner && managed.provider.role == Role::ErrorHandler
    }

    fn return_type(
        input: &Input<'_>,
        list: &std::ops::Range<usize>,
        start: usize,
        method: &str,
        documentation: &str,
    ) -> String {
        if input.extension == "rb" {
            crate::ruby_source::Yard::return_type(documentation)
        } else {
            ReturnType::read(input, start, list.start - 1, list.end + 1, method)
        }
    }

    fn read_status(input: &Input<'_>, declaration: &Declaration) -> Result<u16, Failure> {
        let mut seen = BTreeSet::new();
        declaration
            .annotations
            .iter()
            .try_fold(Self::DEFAULT_STATUS, |status, marker| {
                Self::require(seen.insert(marker.name.as_str()), || {
                    input.failure(marker.span.start, "Duplicate error handler annotation")
                })?;
                Self::status_after(input, marker, status)
            })
    }

    fn status_after(input: &Input<'_>, marker: &Annotation, status: u16) -> Result<u16, Failure> {
        match marker.name.as_str() {
            "handleserror" if marker.arguments.is_empty() => Ok(status),
            "handleserror" => Err(input.failure(marker.span.start,
                "@HandlesError takes no arguments. Declare the handled error type on the method's first parameter")),
            "responsestatus" => Self::response_status(input, marker),
            _ => Err(input.failure(marker.span.start,
                "An @HandlesError method accepts only @HandlesError and @ResponseStatus")),
        }
    }

    fn response_status(input: &Input<'_>, marker: &Annotation) -> Result<u16, Failure> {
        marker
            .arguments
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|value| (400..=599).contains(value))
            .ok_or_else(|| {
                input.failure(
                    marker.span.start,
                    "@ResponseStatus on @HandlesError takes an error status from 400 to 599",
                )
            })
    }

    fn read_parameters(
        input: &Input<'_>,
        list: &std::ops::Range<usize>,
        start: usize,
    ) -> Result<Vec<(String, String)>, Failure> {
        let mut pieces = super::parameters(input, &input.code[list.clone()], start)?;
        Self::remove_receiver(input, &mut pieces, start)?;
        Self::require((1..=2).contains(&pieces.len()), || {
            input.failure(
                start,
                "@HandlesError takes one typed error parameter and optionally the request",
            )
        })?;
        pieces
            .iter()
            .map(|piece| {
                RouteMarker::name_and_type(input.extension, piece)
                    .ok_or_else(|| input.failure(start, Self::type_needed(input.extension)))
            })
            .collect()
    }

    fn remove_receiver(
        input: &Input<'_>,
        pieces: &mut Vec<String>,
        start: usize,
    ) -> Result<(), Failure> {
        let Some(expected) = Self::receiver(input.extension) else {
            return Ok(());
        };
        Self::require(pieces.first().map(String::as_str) == Some(expected), || {
            input.failure(
                start,
                "@HandlesError is an instance method and needs self or &self first",
            )
        })?;
        pieces.remove(0);
        Ok(())
    }

    fn receiver(extension: &str) -> Option<&'static str> {
        Self::RECEIVERS
            .iter()
            .find(|(language, _)| *language == extension)
            .map(|(_, receiver)| *receiver)
    }

    fn type_needed(extension: &str) -> &'static str {
        match extension {
            "js" => Self::JSDOC_TYPE_NEEDED,
            "rb" => Self::YARD_TYPE_NEEDED,
            _ => Self::TYPE_NEEDED,
        }
    }

    fn refuse_static(input: &Input<'_>, start: usize) -> Result<(), Failure> {
        let rest = &input.code[start..];
        let before_parameters = rest.split('(').next().unwrap_or(rest);
        let is_static = super::words(before_parameters).any(|word| word == "static");
        Self::require(!is_static, || input.failure(start, Self::STATIC_METHOD))
    }

    fn error_type(
        input: &Input<'_>,
        parameter: &(String, String),
        documentation: &str,
        start: usize,
    ) -> Result<String, Failure> {
        let value = Self::declared_error_type(input.extension, parameter, documentation);
        let bare = value.trim_start_matches(['&', '*']);
        Self::require(Self::is_error_type(bare), || {
            input.failure(start, Self::type_needed(input.extension))
        })?;
        Self::require(Self::is_borrowed_in_rust(input.extension, &value), || {
            input.failure(
                start,
                "Rust @HandlesError borrows its error: declare the first parameter as &ErrorType",
            )
        })?;
        Ok(value)
    }

    fn declared_error_type(
        extension: &str,
        parameter: &(String, String),
        documentation: &str,
    ) -> String {
        match extension {
            "js" => Self::documented_type(documentation, &parameter.0),
            "rb" => crate::ruby_source::Yard::parameter_type(documentation, &parameter.0),
            _ => parameter.1.trim().to_owned(),
        }
    }

    fn is_error_type(bare: &str) -> bool {
        Self::is_type_name(bare) && !Self::NOT_ERROR_TYPES.contains(&bare)
    }

    fn is_type_name(value: &str) -> bool {
        let only_type_bytes = value.bytes().all(Self::is_type_byte);
        [!value.is_empty(), only_type_bytes] == [true, true]
    }

    fn is_type_byte(byte: u8) -> bool {
        let punctuation = Self::TYPE_NAME_PUNCTUATION.contains(&byte);
        [byte.is_ascii_alphanumeric(), punctuation].contains(&true)
    }

    fn is_borrowed_in_rust(extension: &str, value: &str) -> bool {
        extension != "rs" || value.starts_with('&')
    }

    fn documented_type(documentation: &str, name: &str) -> String {
        documentation
            .match_indices("@param")
            .find_map(|(at, _)| Self::braced_type_for(&documentation[at + 6..], name))
            .unwrap_or_default()
    }

    fn braced_type_for(after_param: &str, name: &str) -> Option<String> {
        let rest = after_param.trim_start().strip_prefix('{')?;
        let (kind, after) = rest.split_once('}')?;
        let names_parameter = after.split_whitespace().next() == Some(name);
        names_parameter.then(|| kind.trim().to_owned())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::Path;

    struct Sources;

    impl Sources {
        const CASES: [(&str, &str, &str); 10] = [
            (
                "js",
                "@RequestRoute\nclass OrdersController {\n@GetRoute\ngetOrder() {}\n}\n@ErrorHandler\nclass OrdersErrorHandler {\n/** @param {Error} error */\n@HandlesError\n@ResponseStatus(404)\nanswer(error) {}\n}\n",
                "Error",
            ),
            (
                "ts",
                "@RequestRoute\nclass OrdersController {\n@GetRoute\ngetOrder() {}\n}\n@ErrorHandler\nclass OrdersErrorHandler {\n@HandlesError\n@ResponseStatus(404)\nanswer(error: Error): string { return ''; }\n}\n",
                "Error",
            ),
            (
                "py",
                "@RequestRoute\nclass OrdersController:\n    @GetRoute\n    def get_order(self): return None\n@ErrorHandler\nclass OrdersErrorHandler:\n    @HandlesError\n    @ResponseStatus(404)\n    def answer(self, error: Exception) -> str: return ''\n",
                "Exception",
            ),
            (
                "java",
                "@RequestRoute\nclass OrdersController {\n@GetRoute\nObject getOrder() { return null; }\n}\n@ErrorHandler\nclass OrdersErrorHandler {\n@HandlesError\n@ResponseStatus(404)\nString answer(Exception error) { return \"\"; }\n}\n",
                "Exception",
            ),
            (
                "cs",
                "[RequestRoute]\nclass OrdersController {\n[GetRoute]\npublic object GetOrder() { return null; }\n}\n[ErrorHandler]\nclass OrdersErrorHandler {\n[HandlesError]\n[ResponseStatus(404)]\npublic string Answer(System.Exception error) { return \"\"; }\n}\n",
                "System.Exception",
            ),
            (
                "kt",
                "@RequestRoute\nclass OrdersController {\n@GetRoute\nfun getOrder(): Any? { return null }\n}\n@ErrorHandler\nclass OrdersErrorHandler {\n@HandlesError\n@ResponseStatus(404)\nfun answer(error: Exception): String { return \"\" }\n}\n",
                "Exception",
            ),
            (
                "php",
                "<?php\n#[RequestRoute]\nclass OrdersController {\n#[GetRoute]\nfunction getOrder() { return null; }\n}\n#[ErrorHandler]\nclass OrdersErrorHandler {\n#[HandlesError]\n#[ResponseStatus(404)]\nfunction answer(Throwable $error): string { return ''; }\n}\n",
                "Throwable",
            ),
            (
                "dart",
                "@RequestRoute\nclass OrdersController {\n@GetRoute\nObject? getOrder() { return null; }\n}\n@ErrorHandler\nclass OrdersErrorHandler {\n@HandlesError\n@ResponseStatus(404)\nString answer(Exception error) { return ''; }\n}\n",
                "Exception",
            ),
            (
                "rs",
                "#[RequestRoute]\nstruct OrdersController;\nimpl OrdersController {\n#[GetRoute]\nfn get_order(&self) -> String { String::new() }\n}\n#[ErrorHandler]\nstruct OrdersErrorHandler;\nimpl OrdersErrorHandler {\n#[HandlesError]\n#[ResponseStatus(404)]\nfn answer(&self, error: &std::io::Error) -> String { String::new() }\n}\n",
                "&std::io::Error",
            ),
            (
                "go",
                "package main\n// @RequestRoute\ntype OrdersController struct{}\n// @GetRoute\nfunc (controller *OrdersController) GetOrder() any { return nil }\n// @ErrorHandler\ntype OrdersErrorHandler struct{}\n// @HandlesError\n// @ResponseStatus(404)\nfunc (handler *OrdersErrorHandler) Answer(error error) string { return \"\" }\n",
                "error",
            ),
        ];
    }

    #[test]
    fn error_parameter_types_are_read_in_every_yon_language() {
        for (extension, source, expected) in Sources::CASES {
            let path = format!("server/controllers/OrdersController.{extension}");
            let source = source
                .replace(
                    "impl OrdersController {",
                    "impl OrdersController {\nfn new() -> Self { Self }\n",
                )
                .replace(
                    "impl OrdersErrorHandler {",
                    "impl OrdersErrorHandler {\nfn new() -> Self { Self }\n",
                );
            let parsed = super::super::parse(Path::new(&path), &source)
                .unwrap_or_else(|failure| panic!("{extension}: {failure}"))
                .expect("annotated plan");
            assert_eq!(parsed.plan.errors.len(), 1, "{extension}");
            let method = &parsed.plan.errors[0];
            assert_eq!(method.error_type, expected, "{extension}");
            assert_eq!(method.status, 404, "{extension}");
            assert!(!method.takes_request, "{extension}");
            assert!(!parsed.source.contains("HandlesError"), "{extension}");
        }
    }

    #[test]
    fn duplicate_types_across_error_handler_classes_are_refused() {
        let source = format!(
            "{}\n@ErrorHandler\nclass OtherErrorHandler {{\n@HandlesError\nanswer(error: Error) {{}}\n}}",
            Sources::CASES[1].1
        );
        let failure = super::super::parse(Path::new("OrdersController.ts"), &source)
            .err()
            .expect("duplicate type");
        assert!(
            failure
                .to_string()
                .contains("Each error type needs one handler")
        );
    }

    struct Refusals;

    impl Refusals {
        const MARKER_ROWS: [(&str, &str, &str); 6] = [
            ("ErrorHandler", "ControllerAdvice", "@ErrorHandler"),
            ("ErrorHandler", "RestControllerAdvice", "@ErrorHandler"),
            ("ErrorHandler", "ExceptionFilter", "@ErrorHandler"),
            ("ErrorHandler", "UseFilters", "@ErrorHandler"),
            ("HandlesError", "ExceptionHandler", "@HandlesError"),
            ("ErrorHandler", "Service", "@ErrorHandler class"),
        ];

        const TYPE_ROWS: [(&str, &str, &str, &str); 44] = [
            (
                "js",
                "/** @param {Error} error */\n",
                "",
                "@param {OrderNotFound} error",
            ),
            ("js", "{Error}", "{*}", "@param {OrderNotFound} error"),
            ("js", "{Error}", "{Object}", "@param {OrderNotFound} error"),
            ("ts", "error: Error", "error", "such as OrderNotFound error"),
            (
                "ts",
                "error: Error",
                "error: any",
                "such as OrderNotFound error",
            ),
            (
                "ts",
                "error: Error",
                "error: Error | TypeError",
                "such as OrderNotFound error",
            ),
            (
                "ts",
                "error: Error",
                "error: string",
                "such as OrderNotFound error",
            ),
            (
                "ts",
                "error: Error)",
                "error: Error, request: any, extra: any)",
                "takes one typed error parameter",
            ),
            (
                "ts",
                "answer(error: Error)",
                "answer()",
                "takes one typed error parameter",
            ),
            ("ts", "answer(", "static answer(", "instance method"),
            (
                "py",
                "error: Exception",
                "error",
                "such as OrderNotFound error",
            ),
            (
                "py",
                "error: Exception",
                "error: object",
                "such as OrderNotFound error",
            ),
            (
                "py",
                "error: Exception",
                "error: Any",
                "such as OrderNotFound error",
            ),
            (
                "py",
                "error: Exception",
                "error: str",
                "such as OrderNotFound error",
            ),
            (
                "py",
                "error: Exception)",
                "error: Exception, request, extra)",
                "takes one typed error parameter",
            ),
            (
                "py",
                "    def answer(self, error",
                "    @staticmethod\n    def answer(error",
                "decorators",
            ),
            (
                "java",
                "Exception error",
                "Object error",
                "such as OrderNotFound error",
            ),
            (
                "java",
                "Exception error",
                "String error",
                "such as OrderNotFound error",
            ),
            (
                "java",
                "Exception error)",
                "Exception error, Request request, Object extra)",
                "takes one typed error parameter",
            ),
            (
                "java",
                "String answer(",
                "static String answer(",
                "instance method",
            ),
            (
                "kt",
                "error: Exception",
                "error: Any",
                "such as OrderNotFound error",
            ),
            (
                "kt",
                "error: Exception",
                "error: String",
                "such as OrderNotFound error",
            ),
            (
                "kt",
                "answer(error: Exception)",
                "answer()",
                "takes one typed error parameter",
            ),
            (
                "cs",
                "System.Exception error",
                "object error",
                "such as OrderNotFound error",
            ),
            (
                "cs",
                "System.Exception error",
                "string error",
                "such as OrderNotFound error",
            ),
            (
                "cs",
                "public string Answer(",
                "public static string Answer(",
                "instance method",
            ),
            (
                "php",
                "Throwable $error",
                "$error",
                "such as OrderNotFound error",
            ),
            (
                "php",
                "Throwable $error",
                "mixed $error",
                "such as OrderNotFound error",
            ),
            (
                "php",
                "Throwable $error",
                "string $error",
                "such as OrderNotFound error",
            ),
            (
                "php",
                "function answer(",
                "static function answer(",
                "instance method",
            ),
            (
                "dart",
                "Exception error",
                "error",
                "such as OrderNotFound error",
            ),
            (
                "dart",
                "Exception error",
                "Object error",
                "such as OrderNotFound error",
            ),
            (
                "dart",
                "Exception error",
                "dynamic error",
                "such as OrderNotFound error",
            ),
            (
                "dart",
                "Exception error",
                "String error",
                "such as OrderNotFound error",
            ),
            (
                "dart",
                "String answer(",
                "static String answer(",
                "instance method",
            ),
            (
                "rs",
                "error: &std::io::Error",
                "error: std::io::Error",
                "borrows its error",
            ),
            ("rs", "&self, error", "error", "instance method"),
            (
                "rs",
                "error: &std::io::Error",
                "error: &String",
                "such as OrderNotFound error",
            ),
            (
                "rs",
                "error: &std::io::Error",
                "error: &dyn std::any::Any",
                "such as OrderNotFound error",
            ),
            (
                "go",
                "error error",
                "error any",
                "such as OrderNotFound error",
            ),
            (
                "go",
                "error error",
                "error string",
                "such as OrderNotFound error",
            ),
            (
                "go",
                "error error",
                "error interface{}",
                "such as OrderNotFound error",
            ),
            (
                "go",
                "func (handler *OrdersErrorHandler) Answer(",
                "func Answer(",
                "Place annotated",
            ),
            (
                "go",
                "error error)",
                "error error, request *http.Request, extra int)",
                "takes one typed error parameter",
            ),
        ];

        fn marker(source: &str, from: &str, to: &str) -> String {
            source
                .replacen(&format!("@{from}\n"), &format!("@{to}\n"), 1)
                .replacen(&format!("[{from}]"), &format!("[{to}]"), 1)
        }

        fn method_marker_line(source: &str) -> &str {
            source
                .lines()
                .find(|line| line.contains("HandlesError"))
                .expect("method marker line")
        }

        fn with_route_marker(source: &str) -> String {
            let line = Self::method_marker_line(source);
            let route = line.replace("HandlesError", "GetRoute");
            source.replacen(line, &format!("{line}\n{route}"), 1)
        }

        fn without_method_markers(source: &str) -> String {
            let mut lines = Vec::new();
            for line in source.lines() {
                if !line.contains("HandlesError") && !line.contains("ResponseStatus") {
                    lines.push(line);
                }
            }
            lines.join("\n")
        }

        fn generic_rows(source: &str) -> Vec<(String, &'static str)> {
            let mut rows = Vec::new();
            for (from, to, fragment) in Self::MARKER_ROWS {
                rows.push((Self::marker(source, from, to), fragment));
            }
            let argument = source.replacen("HandlesError", "HandlesError(\"OrderNotFound\")", 1);
            rows.push((argument, "takes no arguments"));
            rows.push((Self::with_route_marker(source), "@HandlesError"));
            rows.push((
                Self::without_method_markers(source),
                "at least one @HandlesError",
            ));
            let status = source.replacen("ResponseStatus(404)", "ResponseStatus(200)", 1);
            rows.push((status, "400 to 599"));
            rows
        }

        fn parse_message(extension: &str, source: &str) -> Option<String> {
            let path = format!("server/controllers/OrdersController.{extension}");
            let source = source
                .replace(
                    "impl OrdersController {",
                    "impl OrdersController {\nfn new() -> Self { Self }\n",
                )
                .replace(
                    "impl OrdersErrorHandler {",
                    "impl OrdersErrorHandler {\nfn new() -> Self { Self }\n",
                );
            super::super::parse(Path::new(&path), &source)
                .err()
                .map(|failure| failure.to_string())
        }
    }

    #[test]
    fn every_yon_language_refuses_error_handler_misuse_with_a_plain_hint() {
        let mut problems = Vec::new();
        for (extension, source, _) in Sources::CASES {
            for (changed, fragment) in Refusals::generic_rows(source) {
                match Refusals::parse_message(extension, &changed) {
                    None => problems.push(format!("{extension}: accepted: {changed}")),
                    Some(message) if !message.contains(fragment) => {
                        problems.push(format!(
                            "{extension}: message {message:?} lacks {fragment:?}"
                        ));
                    }
                    Some(_) => {}
                }
            }
        }
        for (extension, from, to, fragment) in Refusals::TYPE_ROWS {
            let source = Sources::CASES
                .iter()
                .find(|case| case.0 == extension)
                .expect("language case")
                .1;
            assert!(source.contains(from), "{extension}: {from}");
            let changed = source.replacen(from, to, 1);
            match Refusals::parse_message(extension, &changed) {
                None => problems.push(format!("{extension}: accepted {to:?}")),
                Some(message) if !message.contains(fragment) => {
                    problems.push(format!(
                        "{extension} {to:?}: {message:?} lacks {fragment:?}"
                    ));
                }
                Some(_) => {}
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }
}
