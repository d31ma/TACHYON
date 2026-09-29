use super::{
    Annotation, Declaration, Input, ManagedType, declared_type, matching, method_body,
    method_owner, method_signature, parameters, reject_python_static, validate_streaming_method,
};
use crate::Failure;
use crate::routing::RoutePath;
use crate::yon::plan::{ParameterSource, Route, RouteParameter};
use std::collections::BTreeSet;
use std::ops::Range;

pub(super) struct MappedRoutes {
    pub routes: Vec<Route>,
    pub markers: Vec<Range<usize>>,
}

pub(super) struct RouteMarker;

impl RouteMarker {
    pub(super) const CLASS: &'static str = "requestroute";
    const STATUS: &'static str = "responsestatus";
    const STREAM: &'static str = "stream";
    const MAX_ROUTES: usize = 64;
    const VERBS: [(&'static str, &'static str, &'static str); 6] = [
        ("getroute", "GET", "GetRoute"),
        ("postroute", "POST", "PostRoute"),
        ("putroute", "PUT", "PutRoute"),
        ("patchroute", "PATCH", "PatchRoute"),
        ("deleteroute", "DELETE", "DeleteRoute"),
        ("streamroute", "GET", "StreamRoute"),
    ];
    pub(super) const PHP_REQUEST_ADVICE: &'static str = "PHP reads the request from $_SERVER, $_GET, $_POST, $_COOKIE and php://input, and answers with http_response_code(), header() and echo, so a PHP method takes no request parameter. Remove the parameter and read the superglobals instead";
    const STREAM_ADVICE: &'static str = "@StreamRoute replaces @GetRoute and @Stream: it means the method answers GET requests with a stream of server-sent events. Write @StreamRoute alone and start the method name with 'stream', for example streamTicks. `ty migrate fix --write` converts it";
    const SOURCES: [(&'static str, ParameterSource, &'static str); 5] = [
        ("frompath", ParameterSource::Path, "FromPath"),
        ("fromquery", ParameterSource::Query, "FromQuery"),
        ("frombody", ParameterSource::Body, "FromBody"),
        ("fromheader", ParameterSource::Header, "FromHeader"),
        ("fromcookie", ParameterSource::Cookie, "FromCookie"),
    ];

    pub(super) fn is_method_marker(name: &str) -> bool {
        name == Self::STATUS || Self::verb(name).is_some()
    }

    pub(super) fn is_name(name: &str) -> bool {
        name == Self::CLASS || Self::is_method_marker(name)
    }

    fn verb(name: &str) -> Option<(&'static str, &'static str)> {
        for (marker, verb, written) in Self::VERBS {
            if marker == name {
                return Some((verb, written));
            }
        }
        None
    }

    fn refuse_client_marker(
        input: &Input<'_>,
        at: usize,
        name: &str,
    ) -> Result<Option<ParameterMarker>, Failure> {
        let normalized = name.replace('_', "").to_ascii_lowercase();
        let (instead, meaning) = match normalized.as_str() {
            "pathvariable" => ("FromPath", "the value comes from the path, like {id}"),
            "requestparam" => ("FromQuery", "the value comes from the query string"),
            "requestbody" => ("FromBody", "the value is the request body"),
            "requestheader" => ("FromHeader", "the value comes from a request header"),
            "cookievalue" => ("FromCookie", "the value comes from a cookie"),
            _ => return Ok(None),
        };
        Err(input.failure(
            at,
            format!(
                "On a controller route, @{name} is written @{instead}: it means {meaning}. @{name} belongs on @HTTPClient request methods"
            ),
        ))
    }

    pub(super) fn source(name: &str) -> Option<(ParameterSource, &'static str)> {
        let normalized = name
            .rsplit("::")
            .next()
            .unwrap_or(name)
            .replace('_', "")
            .to_ascii_lowercase();
        for (marker, source, written) in Self::SOURCES {
            if marker == normalized {
                return Some((source, written));
            }
        }
        None
    }
}

pub(super) fn uses_request_route(declarations: &[Declaration]) -> bool {
    declarations.iter().any(|declaration| {
        declaration
            .annotations
            .iter()
            .any(|annotation| annotation.name == RouteMarker::CLASS)
    })
}

pub(super) fn mapped(
    input: &Input<'_>,
    declarations: &[Declaration],
    types: &[ManagedType],
    controller: &str,
) -> Result<MappedRoutes, Failure> {
    let base = base_path(input, declarations)?;
    reject_verb_named_methods(input, types, controller)?;
    let mut found = MappedRoutes {
        routes: Vec::new(),
        markers: Vec::new(),
    };
    let mut seen = BTreeSet::new();
    for declaration in declarations {
        if declared_type(input, declaration).is_some() {
            continue;
        }
        if super::errors::is_method(declaration) {
            continue;
        }
        let route = mapped_route(input, declaration, types, controller, &base, &mut found)?;
        let key = (route.pattern.clone(), route.verb.clone());
        if !seen.insert(key) {
            return Err(input.failure(
                declaration.start,
                format!(
                    "Two methods answer {} {}. Each route path and HTTP method pair needs one method",
                    route.verb,
                    RoutePath::template(route.pattern.as_deref().unwrap_or("/"))
                ),
            ));
        }
        found.routes.push(route);
    }
    if found.routes.is_empty() {
        return Err(input.failure(
            0,
            "A @RequestRoute controller needs at least one route method. Mark a method with @GetRoute, @PostRoute, @PutRoute, @PatchRoute or @DeleteRoute: @GetRoute means the method answers GET requests",
        ));
    }
    if found.routes.len() > RouteMarker::MAX_ROUTES {
        return Err(input.failure(0, "A controller may declare at most 64 route methods"));
    }
    Ok(found)
}

fn base_path(input: &Input<'_>, declarations: &[Declaration]) -> Result<String, Failure> {
    for declaration in declarations {
        for annotation in &declaration.annotations {
            if annotation.name != RouteMarker::CLASS {
                continue;
            }
            if annotation.arguments.is_empty() {
                return Ok(String::new());
            }
            return path_literal(input, annotation, "@RequestRoute");
        }
    }
    Ok(String::new())
}

pub(super) fn path_literal(
    input: &Input<'_>,
    annotation: &Annotation,
    written: &str,
) -> Result<String, Failure> {
    let text = annotation.arguments.trim();
    let quoted = text.len() >= 2
        && ((text.starts_with('"') && text.ends_with('"'))
            || (text.starts_with('\'') && text.ends_with('\'')));
    let inner = text.get(1..text.len().saturating_sub(1)).unwrap_or("");
    if !quoted || inner.contains(['"', '\'', '\\']) {
        return Err(input.failure(
            annotation.span.start,
            format!(
                "{written} takes one path in quotes, for example {written}(\"/orders\"). The path says which URL the code answers"
            ),
        ));
    }
    Ok(inner.to_owned())
}

fn reject_verb_named_methods(
    input: &Input<'_>,
    types: &[ManagedType],
    controller: &str,
) -> Result<(), Failure> {
    let Some(managed) = types
        .iter()
        .find(|managed| managed.provider.name == controller)
    else {
        return Ok(());
    };
    let Some(start) = super::controller_methods(input, managed)?.first().copied() else {
        return Ok(());
    };
    let signature = method_signature(input, start)?;
    let verb = signature.name.to_ascii_lowercase();
    Err(input.failure(
        start,
        format!(
            "A method named {} no longer answers requests by its name. Mark it with @{}Route and give it a name that starts with '{verb}', for example {}Items(): @{}Route means the method answers {} requests",
            signature.name,
            capitalized(&verb),
            verb,
            capitalized(&verb),
            signature.name
        ),
    ))
}

fn capitalized(word: &str) -> String {
    let mut characters = word.chars();
    let first = characters.next().unwrap_or_default().to_ascii_uppercase();
    format!("{first}{}", characters.as_str())
}

struct MethodMarkers {
    verb: &'static str,
    written: &'static str,
    path: String,
    status: Option<u16>,
    stream: bool,
    at: usize,
    documentation: String,
}

fn method_markers(input: &Input<'_>, declaration: &Declaration) -> Result<MethodMarkers, Failure> {
    let mut mapping: Option<(&'static str, &'static str, String)> = None;
    let mut status = None;
    for annotation in &declaration.annotations {
        if annotation.name == RouteMarker::STREAM {
            return Err(input.failure(annotation.span.start, RouteMarker::STREAM_ADVICE));
        }
        if annotation.name == RouteMarker::STATUS {
            status = Some(status_code(input, annotation)?);
            continue;
        }
        if super::access_marker::AccessMarker::is_name(&annotation.name) {
            continue;
        }
        let Some((verb, written)) = RouteMarker::verb(&annotation.name) else {
            return Err(input.failure(
                annotation.span.start,
                "This annotation belongs on a type, not on a method",
            ));
        };
        if mapping.is_some() {
            return Err(input.failure(
                annotation.span.start,
                "A method answers one HTTP method. Keep one of @GetRoute, @PostRoute, @PutRoute, @PatchRoute, @DeleteRoute or @StreamRoute",
            ));
        }
        let path = if annotation.arguments.is_empty() {
            String::new()
        } else {
            path_literal(input, annotation, &format!("@{written}"))?
        };
        mapping = Some((verb, written, path));
    }
    if mapping.is_none() && super::access_marker::AccessMarker::only_access(declaration) {
        return Err(input.failure(
            declaration.start,
            super::access_marker::AccessMarker::METHOD_ADVICE,
        ));
    }
    let Some((verb, written, path)) = mapping else {
        return Err(input.failure(
            declaration.start,
            "@ResponseStatus needs a route annotation on the same method, such as @GetRoute or @PostRoute",
        ));
    };
    Ok(MethodMarkers {
        verb,
        written,
        path,
        status,
        stream: written == "StreamRoute",
        at: declaration.start,
        documentation: documentation_before(input, declaration),
    })
}

fn status_code(input: &Input<'_>, annotation: &Annotation) -> Result<u16, Failure> {
    let code = annotation.arguments.trim().parse::<u16>().ok();
    match code {
        Some(code) if (200..=299).contains(&code) => Ok(code),
        _ => Err(input.failure(
            annotation.span.start,
            "@ResponseStatus takes one success status number from 200 to 299, for example @ResponseStatus(201). It sets the status of a normal answer",
        )),
    }
}

fn mapped_route(
    input: &Input<'_>,
    declaration: &Declaration,
    types: &[ManagedType],
    controller: &str,
    base: &str,
    found: &mut MappedRoutes,
) -> Result<Route, Failure> {
    let markers = method_markers(input, declaration)?;
    let start = declaration.start;
    let owner = method_owner(input, start, types)?;
    if owner != controller {
        return Err(input.failure(
            start,
            format!(
                "@{} belongs on a method of the controller class, not on {owner}",
                markers.written
            ),
        ));
    }
    let signature = method_signature(input, start)?;
    check_verb_prefix(input, start, &signature.name, &markers)?;
    let pattern = RoutePath::pattern(base, &markers.path)
        .map_err(|message| input.failure(markers.at, message))?;
    let mut route = method_route(input, start, &markers, &pattern, found)?;
    route.access = super::access_marker::AccessMarker::read(input, declaration)?;
    Ok(route)
}

pub(super) fn identity_route(
    input: &Input<'_>,
    start: usize,
    pattern: &str,
    found: &mut MappedRoutes,
) -> Result<Route, Failure> {
    let declaration = Declaration {
        annotations: Vec::new(),
        start,
    };
    let markers = MethodMarkers {
        verb: "GET",
        written: "IdentityProvider",
        path: String::new(),
        status: None,
        stream: false,
        at: start,
        documentation: documentation_before(input, &declaration),
    };
    method_route(input, start, &markers, pattern, found)
}

fn method_route(
    input: &Input<'_>,
    start: usize,
    markers: &MethodMarkers,
    pattern: &str,
    found: &mut MappedRoutes,
) -> Result<Route, Failure> {
    let signature = method_signature(input, start)?;
    reject_python_static(input, start)?;
    let list = parameter_list(input, start)?;
    let mut parameters = route_parameters(input, start, &list, pattern, markers, found)?;
    let mut returns = ReturnType::read(input, start, list.start - 1, list.end + 1, &signature.name);
    let documentation = markers.documentation.clone();
    if input.extension == "rb" {
        returns = super::ruby::RubyMethod::read_types(&documentation, &mut parameters);
    }
    if input.extension == "php" {
        returns = crate::route_types::PhpDoc::read_types(&documentation, &mut parameters, returns);
    }
    let body = method_body(input, start, signature.end)?;
    let generator = super::generator_body(input, &body, start) || signature.generator;
    if generator && !markers.stream {
        return Err(input.failure(
            start,
            format!(
                "{} produces many values with yield, so it answers with a stream. {}",
                signature.name,
                RouteMarker::STREAM_ADVICE
            ),
        ));
    }
    let streaming = markers.stream;
    validate_streaming_method(input, start, signature.end, streaming)?;
    let takes_request = parameters
        .iter()
        .any(|parameter| parameter.source == ParameterSource::Request);
    Ok(Route {
        verb: markers.verb.to_owned(),
        method: signature.name,
        streaming,
        socket: None,
        takes_request,
        pattern: Some(pattern.to_owned()),
        status: markers.status,
        parameters,
        returns,
        documentation,
        return_shape: crate::route_types::Shape::Any,
        error_responses: Vec::new(),
        dispatcher: false,
        access: crate::yon::route_access::RouteAccess::Missing,
    })
}

pub(super) struct ReturnType;

impl ReturnType {
    const MODIFIERS: [&'static str; 16] = [
        "public",
        "private",
        "protected",
        "internal",
        "static",
        "async",
        "override",
        "virtual",
        "final",
        "synchronized",
        "abstract",
        "sealed",
        "open",
        "suspend",
        "default",
        "unsafe",
    ];

    pub(super) fn read(
        input: &Input<'_>,
        start: usize,
        opening: usize,
        after: usize,
        name: &str,
    ) -> String {
        match input.extension {
            "java" | "cs" | "dart" => Self::before_name(input, start, opening, name),
            "js" | "rb" => String::new(),
            _ => Self::after_parameters(input, after),
        }
    }

    fn before_name(input: &Input<'_>, start: usize, opening: usize, name: &str) -> String {
        let prefix = input.source[start..opening].trim_end();
        let prefix = prefix.strip_suffix(name).unwrap_or(prefix);
        let mut words: Vec<&str> = prefix.split_whitespace().collect();
        words.retain(|word| !Self::MODIFIERS.contains(word));
        words.join(" ")
    }

    fn after_parameters(input: &Input<'_>, after: usize) -> String {
        let rest = &input.source[after..];
        let text = match input.extension {
            "py" => rest
                .trim_start()
                .strip_prefix("->")
                .map(|value| Self::until(value, &[":"])),
            "rs" => rest
                .trim_start()
                .strip_prefix("->")
                .map(|value| Self::until(value, &["{", "where"])),
            "go" => Some(Self::until(rest, &["{"])),
            _ => rest
                .trim_start()
                .strip_prefix(':')
                .map(|value| Self::until(value, &["{", "=>", "="])),
        };
        text.unwrap_or_default().trim().to_owned()
    }

    fn until<'a>(text: &'a str, stops: &[&str]) -> &'a str {
        let mut end = text.len();
        for stop in stops {
            if let Some(at) = text.find(stop) {
                end = end.min(at);
            }
        }
        &text[..end]
    }
}

pub(super) fn documentation_before(input: &Input<'_>, declaration: &Declaration) -> String {
    if input.extension == "rb" {
        return crate::ruby_source::Yard::comments_before(input.source, declaration.start);
    }
    if !matches!(input.extension, "js" | "php") {
        return String::new();
    }
    let first = declaration
        .annotations
        .iter()
        .map(|annotation| annotation.span.start)
        .min()
        .unwrap_or(declaration.start);
    let before = input.source[..first].trim_end();
    let Some(body) = before.strip_suffix("*/") else {
        return String::new();
    };
    let Some(open) = body.rfind("/**") else {
        return String::new();
    };
    body[open + 3..].to_owned()
}

fn check_verb_prefix(
    input: &Input<'_>,
    start: usize,
    name: &str,
    markers: &MethodMarkers,
) -> Result<(), Failure> {
    let verb = markers
        .written
        .trim_end_matches("Route")
        .to_ascii_lowercase();
    if starts_with_verb(name, &verb) {
        return Ok(());
    }
    let example = match input.extension {
        "py" | "rs" | "rb" => format!("{verb}_items"),
        "cs" | "go" => format!("{}Items", capitalized(&verb)),
        _ => format!("{verb}Items"),
    };
    Err(input.failure(
        start,
        format!(
            "@{} means this method answers {} requests, so its name must start with '{verb}', for example {example}. Rename {name} or change the annotation",
            markers.written, markers.verb
        ),
    ))
}

fn starts_with_verb(name: &str, verb: &str) -> bool {
    let Some(prefix) = name.get(..verb.len()) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case(verb) {
        return false;
    }
    match name[verb.len()..].chars().next() {
        None => true,
        Some(next) => next == '_' || next.is_ascii_uppercase() || next.is_ascii_digit(),
    }
}

pub(super) fn parameter_list(input: &Input<'_>, start: usize) -> Result<Range<usize>, Failure> {
    if input.extension == "rb" {
        return super::ruby::RubyMethod::parameter_list(input, start);
    }
    let suffix = &input.code[start..];
    let search_from = match input.extension {
        "rs" => suffix
            .find("fn ")
            .map_or(start, |offset| start + offset + 3),
        "go" => go_name_start(input, start),
        _ => start,
    };
    let opening = input.code[search_from..]
        .find('(')
        .map(|at| search_from + at)
        .ok_or_else(|| input.failure(start, "A route method needs a parameter list"))?;
    let closing = matching(&input.code, opening, b'(', b')')
        .ok_or_else(|| input.failure(start, "Unbalanced route method parameters"))?;
    Ok(opening + 1..closing)
}

fn go_name_start(input: &Input<'_>, start: usize) -> usize {
    let Some(opening) = input.code[start..].find('(').map(|at| start + at) else {
        return start;
    };
    matching(&input.code, opening, b'(', b')').map_or(start, |closing| closing + 1)
}

fn route_parameters(
    input: &Input<'_>,
    start: usize,
    list: &Range<usize>,
    pattern: &str,
    markers: &MethodMarkers,
    found: &mut MappedRoutes,
) -> Result<Vec<RouteParameter>, Failure> {
    let text = &input.code[list.clone()];
    let pieces = parameters(input, text, start)?;
    let mut result = Vec::new();
    let mut search = list.start;
    for (index, piece) in pieces.iter().enumerate() {
        let at = input.code[search..list.end]
            .find(piece.as_str())
            .map_or(search, |offset| search + offset);
        search = at + piece.len();
        if index == 0 && is_receiver(input.extension, piece, true) {
            continue;
        }
        if index == 0 && matches!(input.extension, "py" | "rs") {
            return Err(input.failure(
                start,
                "Route methods are instance methods: annotated routes require an instance receiver, self in Python or &self in Rust, as the first parameter",
            ));
        }
        let parameter = route_parameter(input, at, piece, found)?;
        result.push(parameter);
    }
    check_parameters(input, start, &result, pattern, markers)?;
    Ok(result)
}

fn is_receiver(extension: &str, piece: &str, first: bool) -> bool {
    if !first {
        return false;
    }
    match extension {
        "py" => piece == "self",
        "rs" => piece == "&self",
        _ => false,
    }
}

fn route_parameter(
    input: &Input<'_>,
    at: usize,
    piece: &str,
    found: &mut MappedRoutes,
) -> Result<RouteParameter, Failure> {
    let original = &input.source[at..at + piece.len()];
    if input.extension == "py"
        && let Some(annotated) =
            crate::python_parameters::PythonAnnotated::read(piece, original, at, |name| {
                RouteMarker::source(name).is_some()
            })
        && let Some((source, written)) = RouteMarker::source(&annotated.marker)
    {
        let name = match &annotated.argument {
            Some(argument) => marker_name(input, at, argument, written)?,
            None => annotated.name.clone(),
        };
        found.markers.extend(annotated.erased);
        return Ok(RouteParameter {
            source,
            name,
            code_name: annotated.name,
            type_text: annotated.type_text,
            shape: crate::route_types::Shape::Any,
        });
    }
    let marker = parameter_marker(input, at, piece, original)?;
    let declaration = match &marker {
        Some(marker) => piece[marker.length..].trim(),
        None => piece.trim(),
    };
    let (code_name, type_text) = name_and_type(input.extension, declaration)
        .ok_or_else(|| input.failure(at, format!("Cannot read the parameter '{piece}'")))?;
    let Some(marker) = marker else {
        return Ok(RouteParameter {
            source: ParameterSource::Request,
            name: code_name.clone(),
            code_name,
            type_text,
            shape: crate::route_types::Shape::Any,
        });
    };
    found.markers.push(at..at + marker.length);
    let name = marker.name.unwrap_or_else(|| code_name.clone());
    Ok(RouteParameter {
        source: marker.source,
        name,
        code_name,
        type_text,
        shape: crate::route_types::Shape::Any,
    })
}

struct ParameterMarker {
    source: ParameterSource,
    name: Option<String>,
    length: usize,
}

fn parameter_marker(
    input: &Input<'_>,
    at: usize,
    piece: &str,
    original: &str,
) -> Result<Option<ParameterMarker>, Failure> {
    let prefix = match input.extension {
        "rs" | "php" | "rb" => "#[",
        "cs" => "[",
        _ => "@",
    };
    let Some(rest) = piece.strip_prefix(prefix) else {
        return Ok(None);
    };
    let length = rest
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
        .count();
    let Some((source, written)) = RouteMarker::source(&rest[..length]) else {
        return RouteMarker::refuse_client_marker(input, at, &rest[..length]);
    };
    if input.extension == "py" {
        return Err(input.failure(
            at,
            crate::annotation_rules::AnnotationRules::python_parameter(written),
        ));
    }
    let mut end = prefix.len() + length;
    let mut name = None;
    if piece[end..].starts_with('(') {
        let closing = matching(piece, end, b'(', b')')
            .ok_or_else(|| input.failure(at, format!("Close the arguments of @{written}")))?;
        name = Some(marker_name(
            input,
            at,
            &original[end + 1..closing],
            written,
        )?);
        end = closing + 1;
    }
    if prefix != "@" {
        if !piece[end..].starts_with(']') {
            return Err(input.failure(at, format!("Close @{written} with ']'")));
        }
        end += 1;
    }
    Ok(Some(ParameterMarker {
        source,
        name,
        length: end,
    }))
}

fn marker_name(input: &Input<'_>, at: usize, text: &str, written: &str) -> Result<String, Failure> {
    let text = text.trim();
    let quoted = text.len() >= 2
        && ((text.starts_with('"') && text.ends_with('"'))
            || (text.starts_with('\'') && text.ends_with('\'')));
    let inner = text.get(1..text.len().saturating_sub(1)).unwrap_or("");
    let valid = !inner.is_empty()
        && inner.len() <= 256
        && inner
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'));
    if quoted && valid {
        return Ok(inner.to_owned());
    }
    Err(input.failure(
        at,
        format!(
            "@{written} takes one name in quotes, for example @{written}(\"page_size\"). Without a name it uses the parameter name"
        ),
    ))
}

pub(super) fn name_and_type(extension: &str, declaration: &str) -> Option<(String, String)> {
    let declaration = declaration.trim();
    match extension {
        "js" | "rb" => simple_name(declaration).map(|name| (name, String::new())),
        "ts" | "py" | "kt" | "rs" => colon_typed(declaration),
        "go" => go_typed(declaration),
        "php" => php_typed(declaration),
        _ => type_first(declaration),
    }
}

fn simple_name(text: &str) -> Option<String> {
    let valid = super::identifier(text) || text == "request";
    valid.then(|| text.to_owned())
}

fn colon_typed(declaration: &str) -> Option<(String, String)> {
    let declaration = declaration.strip_prefix("mut ").unwrap_or(declaration);
    let Some((name, kind)) = declaration.split_once(':') else {
        return simple_name(declaration).map(|name| (name, String::new()));
    };
    let name = name.trim();
    let optional = name.ends_with('?');
    let name = name.trim_end_matches('?');
    let mut kind = kind.trim().to_owned();
    if optional {
        kind = format!("{kind} | undefined");
    }
    simple_name(name).map(|name| (name, kind))
}

fn go_typed(declaration: &str) -> Option<(String, String)> {
    let (name, kind) = declaration.split_once(char::is_whitespace)?;
    simple_name(name).map(|name| (name, kind.trim().to_owned()))
}

fn php_typed(declaration: &str) -> Option<(String, String)> {
    let at = declaration.find('$')?;
    let name = declaration[at + 1..].trim();
    simple_name(name).map(|name| (name, declaration[..at].trim().to_owned()))
}

fn type_first(declaration: &str) -> Option<(String, String)> {
    let declaration = declaration.strip_prefix("final ").unwrap_or(declaration);
    let at = declaration.rfind(char::is_whitespace)?;
    let name = declaration[at..].trim();
    simple_name(name).map(|name| (name, declaration[..at].trim().to_owned()))
}

fn check_parameters(
    input: &Input<'_>,
    start: usize,
    parameters: &[RouteParameter],
    pattern: &str,
    markers: &MethodMarkers,
) -> Result<(), Failure> {
    let mut requests = 0;
    let mut bodies = 0;
    for parameter in parameters {
        if parameter.source == ParameterSource::Request && input.extension == "php" {
            return Err(input.failure(start, RouteMarker::PHP_REQUEST_ADVICE));
        }
        match parameter.source {
            ParameterSource::Request => requests += 1,
            ParameterSource::Body => bodies += 1,
            ParameterSource::Path => check_path_variable(input, start, parameter, pattern)?,
            _ => {}
        }
    }
    if requests > 1 {
        return Err(input.failure(
            start,
            "Mark each route parameter with @FromPath, @FromQuery, @FromBody, @FromHeader or @FromCookie. Only one parameter may be left unmarked, and it receives the whole request",
        ));
    }
    if bodies > 1 {
        return Err(input.failure(
            start,
            "A request has one body, so only one parameter may use @FromBody",
        ));
    }
    if bodies == 1 && markers.verb == "GET" {
        return Err(input.failure(
            start,
            "@GetRoute means this method answers GET requests, and a GET request has no body. Remove @FromBody or use @PostRoute",
        ));
    }
    Ok(())
}

fn check_path_variable(
    input: &Input<'_>,
    start: usize,
    parameter: &RouteParameter,
    pattern: &str,
) -> Result<(), Failure> {
    let wanted = format!("_{}", parameter.name);
    if pattern.split('/').any(|segment| segment == wanted) {
        return Ok(());
    }
    Err(input.failure(
        start,
        format!(
            "@FromPath means the value comes from the route path, but '{}' has no {{{}}} part. Add {{{}}} to the path or use another marker",
            RoutePath::template(pattern),
            parameter.name,
            parameter.name
        ),
    ))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::starts_with_verb;
    use crate::yon::plan::{ParameterSource, Route};
    use std::path::Path;

    struct Sample;

    impl Sample {
        const CONTROLLERS: [(&'static str, &'static str); 10] = [
            (
                "js",
                "@RequestRoute(\"/orders\")\nclass OrdersController {\n  @GetRoute(\"/{id}\")\n  getOrder(@FromPath id, @FromQuery(\"view\") view) { return {} }\n}\n",
            ),
            (
                "ts",
                "@RequestRoute(\"/orders\")\nclass OrdersController {\n  @GetRoute(\"/{id}\")\n  getOrder(@FromPath id: number, @FromQuery(\"view\") view: string) { return {} }\n}\n",
            ),
            (
                "py",
                "from typing import Annotated\n\n@request_route(\"/orders\")\nclass OrdersController:\n    @get_route(\"/{id}\")\n    def get_order(self, id: Annotated[int, from_path], view: Annotated[str, from_query(\"view\")]):\n        return {}\n",
            ),
            (
                "php",
                "<?php\n#[RequestRoute(\"/orders\")]\nclass OrdersController {\n    #[GetRoute(\"/{id}\")]\n    public function getOrder(#[FromPath] int $id, #[FromQuery(\"view\")] string $view) { return []; }\n}\n",
            ),
            (
                "java",
                "@RequestRoute(\"/orders\")\nclass OrdersController {\n    @GetRoute(\"/{id}\")\n    public Object getOrder(@FromPath long id, @FromQuery(\"view\") String view) { return null; }\n}\n",
            ),
            (
                "kt",
                "@RequestRoute(\"/orders\")\nclass OrdersController {\n    @GetRoute(\"/{id}\")\n    fun getOrder(@FromPath id: Long, @FromQuery(\"view\") view: String): Any? { return null }\n}\n",
            ),
            (
                "cs",
                "[RequestRoute(\"/orders\")]\nclass OrdersController {\n    [GetRoute(\"/{id}\")]\n    public object GetOrder([FromPath] long id, [FromQuery(\"view\")] string view) { return null; }\n}\n",
            ),
            (
                "rs",
                "#[RequestRoute(\"/orders\")]\nstruct OrdersController {}\nimpl OrdersController {\n    fn new() -> Self { Self {} }\n    #[GetRoute(\"/{id}\")]\n    fn get_order(&self, #[FromPath] id: i64, #[FromQuery(\"view\")] view: String) -> Json { Json::Null }\n}\n",
            ),
            (
                "dart",
                "@RequestRoute(\"/orders\")\nclass OrdersController {\n  @GetRoute(\"/{id}\")\n  Object? getOrder(@FromPath int id, @FromQuery(\"view\") String view) {\n    return null;\n  }\n}\n",
            ),
            (
                "go",
                "// @RequestRoute(\"/orders\")\ntype OrdersController struct {\n}\n\n// @GetRoute(\"/{id}\")\nfunc (controller *OrdersController) GetOrder(\n// @FromPath\n id int64, \n// @FromQuery(\"view\")\n view string) any {\n\treturn nil\n}\n",
            ),
        ];

        fn routes(extension: &str, source: &str) -> (Vec<Route>, String) {
            let path = format!("server/controllers/OrdersController.{extension}");
            let parsed = super::super::parse(Path::new(&path), source)
                .unwrap_or_else(|failure| panic!("{extension}: {failure}"))
                .expect("controller");
            (parsed.plan.routes, parsed.source)
        }
    }

    #[test]
    fn every_yon_language_reads_route_methods_and_their_parameters() {
        for (extension, source) in Sample::CONTROLLERS {
            let (routes, stripped) = Sample::routes(extension, source);
            assert_eq!(routes.len(), 1, "{extension}");
            let route = &routes[0];
            assert_eq!(route.verb, "GET", "{extension}");
            assert_eq!(route.pattern.as_deref(), Some("/orders/_id"), "{extension}");
            assert_eq!(route.parameters.len(), 2, "{extension}");
            assert_eq!(
                route.parameters[0].source,
                ParameterSource::Path,
                "{extension}"
            );
            assert_eq!(route.parameters[0].name, "id", "{extension}");
            assert_eq!(
                route.parameters[1].source,
                ParameterSource::Query,
                "{extension}"
            );
            assert_eq!(route.parameters[1].name, "view", "{extension}");
            assert_eq!(route.parameters[1].code_name, "view", "{extension}");
            for marker in [
                "FromPath",
                "from_path",
                "FromQuery",
                "from_query",
                "GetRoute",
                "RequestRoute",
                "request_route",
                "get_route",
            ] {
                assert!(!stripped.contains(marker), "{extension}: {stripped}");
            }
        }
    }

    #[test]
    fn typed_languages_keep_the_parameter_types() {
        let expected = [
            ("ts", "number"),
            ("py", "int"),
            ("php", "int"),
            ("java", "long"),
            ("kt", "Long"),
            ("cs", "long"),
            ("rs", "i64"),
            ("dart", "int"),
            ("go", "int64"),
        ];
        for (extension, source) in Sample::CONTROLLERS {
            let Some((_, kind)) = expected.iter().find(|(language, _)| *language == extension)
            else {
                continue;
            };
            let (routes, _) = Sample::routes(extension, source);
            assert_eq!(routes[0].parameters[0].type_text, *kind, "{extension}");
        }
    }

    #[test]
    fn a_method_name_starts_with_its_verb_in_any_casing() {
        for name in ["get", "getOrder", "GetOrder", "get_order", "GET", "get2"] {
            assert!(starts_with_verb(name, "get"), "{name}");
        }
        for name in ["getter", "target", "ge", "post"] {
            assert!(!starts_with_verb(name, "get"), "{name}");
        }
    }
}
