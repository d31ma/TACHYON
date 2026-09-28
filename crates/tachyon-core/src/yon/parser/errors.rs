use super::{Declaration, Input, ManagedType, method_body, method_owner, method_signature};
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
    let mut methods = Vec::new();
    let mut handled = BTreeSet::new();
    for declaration in declarations {
        if !is_method(declaration) {
            continue;
        }
        let method = read_method(input, declaration, types)?;
        let key = if input.extension == "php" {
            method.error_type.to_ascii_lowercase()
        } else {
            method.error_type.clone()
        };
        if !handled.insert(key) {
            return Err(input.failure(declaration.start, format!(
                "Two @HandlesError methods handle {}. Each error type needs one handler in the project",
                method.error_type
            )));
        }
        methods.push(method);
    }
    for managed in types {
        if managed.provider.role == Role::ErrorHandler
            && !methods
                .iter()
                .any(|method| method.owner == managed.provider.name)
        {
            return Err(input.failure(
                managed.declaration,
                "An @ErrorHandler class needs at least one @HandlesError instance method",
            ));
        }
    }
    Ok(methods)
}

fn read_method(
    input: &Input<'_>,
    declaration: &Declaration,
    types: &[ManagedType],
) -> Result<ErrorMethod, Failure> {
    let start = declaration.start;
    let owner = method_owner(input, start, types)?;
    if !types.iter().any(|managed| {
        managed.provider.name == owner && managed.provider.role == Role::ErrorHandler
    }) {
        return Err(input.failure(
            start,
            "@HandlesError belongs on a method of an @ErrorHandler class",
        ));
    }
    let status = read_status(input, declaration)?;
    refuse_static(input, start)?;
    let signature = method_signature(input, start)?;
    super::reject_python_static(input, start)?;
    let body = method_body(input, start, signature.end)?;
    if signature.generator || super::generator_body(input, &body, start) {
        return Err(input.failure(
            start,
            "@HandlesError returns one response and cannot stream",
        ));
    }
    let list = super::routes::parameter_list(input, start)?;
    let documentation = super::routes::documentation_before(input, declaration);
    let parameters = read_parameters(input, &list, start)?;
    if parameters.len() == 2 && input.extension == "php" {
        return Err(input.failure(start, super::routes::RouteMarker::PHP_REQUEST_ADVICE));
    }
    let error_type = error_type(input, &parameters[0], &documentation, start)?;
    let mut returns = super::routes::ReturnType::read(
        input,
        start,
        list.start - 1,
        list.end + 1,
        &signature.name,
    );
    if input.extension == "rb" {
        returns = crate::ruby_source::Yard::return_type(&documentation);
    }
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

fn read_status(input: &Input<'_>, declaration: &Declaration) -> Result<u16, Failure> {
    let mut seen = BTreeSet::new();
    let mut status = 500;
    for marker in &declaration.annotations {
        if !seen.insert(marker.name.as_str()) {
            return Err(input.failure(marker.span.start, "Duplicate error handler annotation"));
        }
        match marker.name.as_str() {
            "handleserror" if marker.arguments.is_empty() => {}
            "handleserror" => return Err(input.failure(marker.span.start,
                "@HandlesError takes no arguments. Declare the handled error type on the method's first parameter")),
            "responsestatus" => {
                status = marker.arguments.trim().parse::<u16>().ok()
                    .filter(|value| (400..=599).contains(value))
                    .ok_or_else(|| input.failure(marker.span.start,
                        "@ResponseStatus on @HandlesError takes an error status from 400 to 599"))?;
            }
            _ => return Err(input.failure(marker.span.start,
                "An @HandlesError method accepts only @HandlesError and @ResponseStatus")),
        }
    }
    Ok(status)
}

fn read_parameters(
    input: &Input<'_>,
    list: &std::ops::Range<usize>,
    start: usize,
) -> Result<Vec<(String, String)>, Failure> {
    let mut pieces = super::parameters(input, &input.code[list.clone()], start)?;
    if matches!(input.extension, "py" | "rs") {
        let expected = if input.extension == "py" {
            "self"
        } else {
            "&self"
        };
        if pieces.first().map(String::as_str) != Some(expected) {
            return Err(input.failure(
                start,
                "@HandlesError is an instance method and needs self or &self first",
            ));
        }
        pieces.remove(0);
    }
    if !(1..=2).contains(&pieces.len()) {
        return Err(input.failure(
            start,
            "@HandlesError takes one typed error parameter and optionally the request",
        ));
    }
    let mut parameters = Vec::new();
    for piece in pieces {
        let parameter = super::routes::name_and_type(input.extension, &piece)
            .ok_or_else(|| input.failure(start, ErrorRules::type_needed(input.extension)))?;
        parameters.push(parameter);
    }
    Ok(parameters)
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

    const TYPE_NEEDED: &'static str = "@HandlesError needs one error type on its first parameter, such as OrderNotFound error. The type must be a native error class or type. any, Object, unions and plain values are refused";

    const JSDOC_TYPE_NEEDED: &'static str = "@HandlesError needs one error type for its first parameter. Add @param {OrderNotFound} error to the method's JSDoc. The type must be an Error class";

    const YARD_TYPE_NEEDED: &'static str = "#[HandlesError] needs one error type for its first parameter. Add `# @param error [OrderNotFound]` to the method's YARD comment. The type must be a StandardError class";

    const STATIC_METHOD: &'static str =
        "@HandlesError goes on an instance method. Remove static from the method";

    fn type_needed(extension: &str) -> &'static str {
        if extension == "js" {
            return Self::JSDOC_TYPE_NEEDED;
        }
        if extension == "rb" {
            return Self::YARD_TYPE_NEEDED;
        }
        Self::TYPE_NEEDED
    }

    fn is_type_name(value: &str) -> bool {
        if value.is_empty() {
            return false;
        }
        value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'\\'))
    }
}

fn refuse_static(input: &Input<'_>, start: usize) -> Result<(), Failure> {
    let rest = &input.code[start..];
    let before_parameters = rest.split('(').next().unwrap_or(rest);
    if super::words(before_parameters).any(|word| word == "static") {
        return Err(input.failure(start, ErrorRules::STATIC_METHOD));
    }
    Ok(())
}

fn error_type(
    input: &Input<'_>,
    parameter: &(String, String),
    documentation: &str,
    start: usize,
) -> Result<String, Failure> {
    let value = if input.extension == "js" {
        documented_type(documentation, &parameter.0)
    } else if input.extension == "rb" {
        crate::ruby_source::Yard::parameter_type(documentation, &parameter.0)
    } else {
        parameter.1.trim().to_owned()
    };
    let bare = value.trim_start_matches(['&', '*']);
    if !ErrorRules::is_type_name(bare) || ErrorRules::NOT_ERROR_TYPES.contains(&bare) {
        return Err(input.failure(start, ErrorRules::type_needed(input.extension)));
    }
    if input.extension == "rs" && !value.starts_with('&') {
        return Err(input.failure(
            start,
            "Rust @HandlesError borrows its error: declare the first parameter as &ErrorType",
        ));
    }
    Ok(value)
}

fn documented_type(documentation: &str, name: &str) -> String {
    for (at, _) in documentation.match_indices("@param") {
        let rest = documentation[at + 6..].trim_start();
        let Some(rest) = rest.strip_prefix('{') else {
            continue;
        };
        let Some((kind, after)) = rest.split_once('}') else {
            continue;
        };
        if after.split_whitespace().next() == Some(name) {
            return kind.trim().to_owned();
        }
    }
    String::new()
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
