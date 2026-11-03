use super::{
    Annotation, Declaration, Input, ManagedType, declared_type, matching, method_body,
    method_owner, method_signature, parameters, reject_python_static, validate_streaming_method,
};
use crate::Failure;
use crate::python_parameters::PythonAnnotated;
use crate::routing::RoutePath;
use crate::yon::plan::{ParameterSource, Route, RouteParameter};
use std::collections::BTreeSet;
use std::ops::Range;

pub(super) struct MappedRoutes {
    pub routes: Vec<Route>,
    pub markers: Vec<Range<usize>>,
}

impl MappedRoutes {
    const RECEIVERS: [(&'static str, &'static str); 2] = [("py", "self"), ("rs", "&self")];

    fn base_path(input: &Input<'_>, declarations: &[Declaration]) -> Result<String, Failure> {
        declarations
            .iter()
            .flat_map(|declaration| &declaration.annotations)
            .find(|annotation| annotation.name == RouteMarker::CLASS)
            .filter(|annotation| !annotation.arguments.is_empty())
            .map_or_else(
                || Ok(String::new()),
                |annotation| RouteMarker::path_literal(input, annotation, "@RequestRoute"),
            )
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
        let first_method = super::controller_methods(input, managed)?.first().copied();
        first_method.map_or(Ok(()), |start| Self::refuse_verb_named_method(input, start))
    }

    fn refuse_verb_named_method(input: &Input<'_>, start: usize) -> Result<(), Failure> {
        let signature = method_signature(input, start)?;
        let verb = signature.name.to_ascii_lowercase();
        Err(input.failure(
            start,
            format!(
                "A method named {} no longer answers requests by its name. Mark it with @{}Route and give it a name that starts with '{verb}', for example {}Items(): @{}Route means the method answers {} requests",
                signature.name,
                RouteMarker::capitalized(&verb),
                verb,
                RouteMarker::capitalized(&verb),
                signature.name
            ),
        ))
    }

    fn add_route(
        &mut self,
        input: &Input<'_>,
        declaration: &Declaration,
        types: &[ManagedType],
        controller: &str,
        base: &str,
        seen: &mut BTreeSet<(Option<String>, String)>,
    ) -> Result<(), Failure> {
        let route = self.mapped_route(input, declaration, types, controller, base)?;
        let first_route = seen.insert((route.pattern.clone(), route.verb.clone()));
        RouteMarker::require(first_route, || {
            input.failure(
                declaration.start,
                format!(
                    "Two methods answer {} {}. Each route path and HTTP method pair needs one method",
                    route.verb,
                    RoutePath::template(route.pattern.as_deref().unwrap_or("/"))
                ),
            )
        })?;
        self.routes.push(route);
        Ok(())
    }

    fn check_route_count(&self, input: &Input<'_>) -> Result<(), Failure> {
        RouteMarker::require(!self.routes.is_empty(), || {
            input.failure(
                0,
                "A @RequestRoute controller needs at least one route method. Mark a method with @GetRoute, @PostRoute, @PutRoute, @PatchRoute or @DeleteRoute: @GetRoute means the method answers GET requests",
            )
        })?;
        RouteMarker::require(self.routes.len() <= RouteMarker::MAX_ROUTES, || {
            input.failure(0, "A controller may declare at most 64 route methods")
        })
    }

    fn mapped_route(
        &mut self,
        input: &Input<'_>,
        declaration: &Declaration,
        types: &[ManagedType],
        controller: &str,
        base: &str,
    ) -> Result<Route, Failure> {
        let markers = MethodMarkers::of_declaration(input, declaration)?;
        let start = declaration.start;
        let owner = method_owner(input, start, types)?;
        RouteMarker::require(owner == controller, || {
            input.failure(
                start,
                format!(
                    "@{} belongs on a method of the controller class, not on {owner}",
                    markers.written
                ),
            )
        })?;
        let signature = method_signature(input, start)?;
        markers.check_verb_prefix(input, start, &signature.name)?;
        let pattern = RoutePath::pattern(base, &markers.path)
            .map_err(|message| input.failure(markers.at, message))?;
        let mut route = self.method_route(input, start, &markers, &pattern)?;
        route.access = super::access_marker::AccessMarker::read(input, declaration)?;
        Ok(route)
    }

    fn method_route(
        &mut self,
        input: &Input<'_>,
        start: usize,
        markers: &MethodMarkers,
        pattern: &str,
    ) -> Result<Route, Failure> {
        let signature = method_signature(input, start)?;
        reject_python_static(input, start)?;
        let list = RouteMarker::parameter_list(input, start)?;
        let mut parameters = self.route_parameters(input, start, &list, pattern, markers)?;
        let written_returns =
            ReturnType::read(input, start, list.start - 1, list.end + 1, &signature.name);
        let documentation = markers.documentation.clone();
        let returns = ReturnType::documented(
            input.extension,
            &documentation,
            &mut parameters,
            written_returns,
        );
        let body = method_body(input, start, signature.end)?;
        markers.refuse_unmarked_stream(
            input,
            start,
            &signature.name,
            signature.generator,
            &body,
        )?;
        let streaming = markers.stream;
        validate_streaming_method(input, start, signature.end, streaming, &returns)?;
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

    fn route_parameters(
        &mut self,
        input: &Input<'_>,
        start: usize,
        list: &Range<usize>,
        pattern: &str,
        markers: &MethodMarkers,
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
            result.extend(self.declared_parameter(input, start, index, at, piece)?);
        }
        markers.check_parameters(input, start, &result, pattern)?;
        Ok(result)
    }

    fn declared_parameter(
        &mut self,
        input: &Input<'_>,
        start: usize,
        index: usize,
        at: usize,
        piece: &str,
    ) -> Result<Option<RouteParameter>, Failure> {
        let receiver = (index == 0)
            .then(|| Self::receiver(input.extension))
            .flatten();
        match receiver {
            Some(receiver) if piece == receiver => Ok(None),
            Some(_) => Err(input.failure(
                start,
                "Route methods are instance methods: annotated routes require an instance receiver, self in Python or &self in Rust, as the first parameter",
            )),
            None => self.route_parameter(input, at, piece).map(Some),
        }
    }

    fn receiver(extension: &str) -> Option<&'static str> {
        Self::RECEIVERS
            .iter()
            .find(|(language, _)| *language == extension)
            .map(|(_, receiver)| *receiver)
    }

    fn route_parameter(
        &mut self,
        input: &Input<'_>,
        at: usize,
        piece: &str,
    ) -> Result<RouteParameter, Failure> {
        let (parameter, erased) = Self::python_annotated(input, at, piece).map_or_else(
            || Self::marked_parameter(input, at, piece),
            |(annotated, found)| Self::python_parameter(input, at, annotated, found),
        )?;
        self.markers.extend(erased);
        Ok(parameter)
    }

    fn python_annotated(
        input: &Input<'_>,
        at: usize,
        piece: &str,
    ) -> Option<(PythonAnnotated, (ParameterSource, &'static str))> {
        let original = &input.source[at..at + piece.len()];
        let annotated = (input.extension == "py")
            .then(|| {
                PythonAnnotated::read(piece, original, at, |name| {
                    RouteMarker::source(name).is_some()
                })
            })
            .flatten()?;
        let found = RouteMarker::source(&annotated.marker)?;
        Some((annotated, found))
    }

    fn python_parameter(
        input: &Input<'_>,
        at: usize,
        annotated: PythonAnnotated,
        (source, written): (ParameterSource, &'static str),
    ) -> Result<(RouteParameter, Vec<Range<usize>>), Failure> {
        let name = annotated.argument.as_ref().map_or_else(
            || Ok(annotated.name.clone()),
            |argument| ParameterMarker::marker_name(input, at, argument, written),
        )?;
        let parameter = RouteParameter {
            source,
            name,
            code_name: annotated.name,
            type_text: annotated.type_text,
            shape: crate::route_types::Shape::Any,
        };
        Ok((parameter, Vec::from(annotated.erased)))
    }

    fn marked_parameter(
        input: &Input<'_>,
        at: usize,
        piece: &str,
    ) -> Result<(RouteParameter, Vec<Range<usize>>), Failure> {
        let marker = ParameterMarker::read_marker(input, at, piece)?;
        let declaration = marker
            .as_ref()
            .map_or(piece, |marker| &piece[marker.length..])
            .trim();
        let (code_name, type_text) = RouteMarker::name_and_type(input.extension, declaration)
            .ok_or_else(|| input.failure(at, format!("Cannot read the parameter '{piece}'")))?;
        let source = marker
            .as_ref()
            .map_or(ParameterSource::Request, |marker| marker.source);
        let erased = marker
            .as_ref()
            .map(|marker| at..at + marker.length)
            .into_iter()
            .collect();
        let name = marker
            .and_then(|marker| marker.name)
            .unwrap_or_else(|| code_name.clone());
        let parameter = RouteParameter {
            source,
            name,
            code_name,
            type_text,
            shape: crate::route_types::Shape::Any,
        };
        Ok((parameter, erased))
    }
}

pub(super) struct RouteMarker;

impl RouteMarker {
    pub(super) const CLASS: &'static str = "requestroute";
    const STATUS: &'static str = "responsestatus";
    const STREAM: &'static str = "stream";
    const MAX_ROUTES: usize = 64;
    const QUOTES: [char; 2] = ['"', '\''];
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
        [name == Self::STATUS, Self::verb(name).is_some()].contains(&true)
    }

    pub(super) fn is_name(name: &str) -> bool {
        [name == Self::CLASS, Self::is_method_marker(name)].contains(&true)
    }

    fn verb(name: &str) -> Option<(&'static str, &'static str)> {
        Self::VERBS
            .iter()
            .find(|(marker, _, _)| *marker == name)
            .map(|(_, verb, written)| (*verb, *written))
    }

    fn require(condition: bool, refusal: impl FnOnce() -> Failure) -> Result<(), Failure> {
        condition.then_some(()).ok_or_else(refusal)
    }

    fn capitalized(word: &str) -> String {
        let mut characters = word.chars();
        let first = characters.next().unwrap_or_default().to_ascii_uppercase();
        format!("{first}{}", characters.as_str())
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
        Self::SOURCES
            .iter()
            .find(|(marker, _, _)| *marker == normalized)
            .map(|(_, source, written)| (*source, *written))
    }

    pub(super) fn path_literal(
        input: &Input<'_>,
        annotation: &Annotation,
        written: &str,
    ) -> Result<String, Failure> {
        Self::quoted_inner(annotation.arguments.trim())
            .filter(|inner| !inner.contains(['"', '\'', '\\']))
            .map(str::to_owned)
            .ok_or_else(|| {
                input.failure(
                    annotation.span.start,
                    format!(
                        "{written} takes one path in quotes, for example {written}(\"/orders\"). The path says which URL the code answers"
                    ),
                )
            })
    }

    fn quoted_inner(text: &str) -> Option<&str> {
        let wrapped = Self::QUOTES
            .iter()
            .any(|quote| Self::is_wrapped_in(text, *quote));
        let quoted = [text.len() >= 2, wrapped] == [true, true];
        quoted.then(|| &text[1..text.len() - 1])
    }

    fn is_wrapped_in(text: &str, quote: char) -> bool {
        text.starts_with(quote) && text.ends_with(quote)
    }

    pub(super) fn documentation_before(input: &Input<'_>, declaration: &Declaration) -> String {
        match input.extension {
            "rb" => crate::ruby_source::Yard::comments_before(input.source, declaration.start),
            "js" | "php" => Self::doc_comment_before(input, declaration),
            _ => String::new(),
        }
    }

    fn doc_comment_before(input: &Input<'_>, declaration: &Declaration) -> String {
        let first = declaration
            .annotations
            .iter()
            .map(|annotation| annotation.span.start)
            .min()
            .unwrap_or(declaration.start);
        let before = input.source[..first].trim_end();
        before
            .strip_suffix("*/")
            .and_then(|body| body.rfind("/**").map(|open| body[open + 3..].to_owned()))
            .unwrap_or_default()
    }

    pub(super) fn parameter_list(input: &Input<'_>, start: usize) -> Result<Range<usize>, Failure> {
        match input.extension {
            "rb" => super::ruby::RubyMethod::parameter_list(input, start),
            _ => Self::bracketed_parameter_list(input, start),
        }
    }

    fn bracketed_parameter_list(input: &Input<'_>, start: usize) -> Result<Range<usize>, Failure> {
        let search_from = Self::parameters_search_start(input, start);
        let opening = input.code[search_from..]
            .find('(')
            .map(|at| search_from + at)
            .ok_or_else(|| input.failure(start, "A route method needs a parameter list"))?;
        let closing = matching(&input.code, opening, b'(', b')')
            .ok_or_else(|| input.failure(start, "Unbalanced route method parameters"))?;
        Ok(opening + 1..closing)
    }

    fn parameters_search_start(input: &Input<'_>, start: usize) -> usize {
        let suffix = &input.code[start..];
        match input.extension {
            "rs" => suffix
                .find("fn ")
                .map_or(start, |offset| start + offset + 3),
            "go" => Self::go_name_start(input, start),
            _ => start,
        }
    }

    fn go_name_start(input: &Input<'_>, start: usize) -> usize {
        input.code[start..]
            .find('(')
            .map(|at| start + at)
            .and_then(|opening| matching(&input.code, opening, b'(', b')'))
            .map_or(start, |closing| closing + 1)
    }

    pub(super) fn name_and_type(extension: &str, declaration: &str) -> Option<(String, String)> {
        let declaration = declaration.trim();
        match extension {
            "js" | "rb" => Self::untyped(declaration),
            "ts" | "py" | "kt" | "rs" => Self::colon_typed(declaration),
            "go" => Self::go_typed(declaration),
            "php" => Self::php_typed(declaration),
            _ => Self::type_first(declaration),
        }
    }

    fn simple_name(text: &str) -> Option<String> {
        let valid = [super::identifier(text), text == "request"].contains(&true);
        valid.then(|| text.to_owned())
    }

    fn untyped(declaration: &str) -> Option<(String, String)> {
        Self::simple_name(declaration).map(|name| (name, String::new()))
    }

    fn colon_typed(declaration: &str) -> Option<(String, String)> {
        let declaration = declaration.strip_prefix("mut ").unwrap_or(declaration);
        declaration.split_once(':').map_or_else(
            || Self::untyped(declaration),
            |(name, kind)| Self::colon_typed_pair(name, kind),
        )
    }

    fn colon_typed_pair(name: &str, kind: &str) -> Option<(String, String)> {
        let name = name.trim();
        let kind = Self::optional_kind(kind.trim(), name.ends_with('?'));
        Self::simple_name(name.trim_end_matches('?')).map(|name| (name, kind))
    }

    fn optional_kind(kind: &str, optional: bool) -> String {
        if optional {
            format!("{kind} | undefined")
        } else {
            kind.to_owned()
        }
    }

    fn go_typed(declaration: &str) -> Option<(String, String)> {
        let (name, kind) = declaration.split_once(char::is_whitespace)?;
        Self::simple_name(name).map(|name| (name, kind.trim().to_owned()))
    }

    fn php_typed(declaration: &str) -> Option<(String, String)> {
        let at = declaration.find('$')?;
        let name = declaration[at + 1..].trim();
        Self::simple_name(name).map(|name| (name, declaration[..at].trim().to_owned()))
    }

    fn type_first(declaration: &str) -> Option<(String, String)> {
        let declaration = declaration.strip_prefix("final ").unwrap_or(declaration);
        let at = declaration.rfind(char::is_whitespace)?;
        let name = declaration[at..].trim();
        Self::simple_name(name).map(|name| (name, declaration[..at].trim().to_owned()))
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
    let base = MappedRoutes::base_path(input, declarations)?;
    MappedRoutes::reject_verb_named_methods(input, types, controller)?;
    let mut found = MappedRoutes {
        routes: Vec::new(),
        markers: Vec::new(),
    };
    let mut seen = BTreeSet::new();
    declarations
        .iter()
        .filter(|declaration| declared_type(input, declaration).is_none())
        .filter(|declaration| !super::errors::is_method(declaration))
        .try_for_each(|declaration| {
            found.add_route(input, declaration, types, controller, &base, &mut seen)
        })?;
    found.check_route_count(input)?;
    Ok(found)
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
        documentation: RouteMarker::documentation_before(input, &declaration),
    };
    found.method_route(input, start, &markers, pattern)
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

impl MethodMarkers {
    fn of_declaration(input: &Input<'_>, declaration: &Declaration) -> Result<Self, Failure> {
        let mut mapping = None;
        let mut status = None;
        declaration.annotations.iter().try_for_each(|annotation| {
            Self::read_annotation(input, annotation, &mut mapping, &mut status)
        })?;
        let (verb, written, path) =
            mapping.ok_or_else(|| Self::missing_mapping(input, declaration))?;
        Ok(Self {
            verb,
            written,
            path,
            status,
            stream: written == "StreamRoute",
            at: declaration.start,
            documentation: RouteMarker::documentation_before(input, declaration),
        })
    }

    fn read_annotation(
        input: &Input<'_>,
        annotation: &Annotation,
        mapping: &mut Option<(&'static str, &'static str, String)>,
        status: &mut Option<u16>,
    ) -> Result<(), Failure> {
        if annotation.name == RouteMarker::STREAM {
            Err(input.failure(annotation.span.start, RouteMarker::STREAM_ADVICE))
        } else if annotation.name == RouteMarker::STATUS {
            *status = Some(Self::status_code(input, annotation)?);
            Ok(())
        } else if super::access_marker::AccessMarker::is_name(&annotation.name) {
            Ok(())
        } else {
            Self::read_mapping(input, annotation, mapping)
        }
    }

    fn read_mapping(
        input: &Input<'_>,
        annotation: &Annotation,
        mapping: &mut Option<(&'static str, &'static str, String)>,
    ) -> Result<(), Failure> {
        let (verb, written) = RouteMarker::verb(&annotation.name).ok_or_else(|| {
            input.failure(
                annotation.span.start,
                "This annotation belongs on a type, not on a method",
            )
        })?;
        RouteMarker::require(mapping.is_none(), || {
            input.failure(
                annotation.span.start,
                "A method answers one HTTP method. Keep one of @GetRoute, @PostRoute, @PutRoute, @PatchRoute, @DeleteRoute or @StreamRoute",
            )
        })?;
        let path = Self::mapping_path(input, annotation, written)?;
        *mapping = Some((verb, written, path));
        Ok(())
    }

    fn mapping_path(
        input: &Input<'_>,
        annotation: &Annotation,
        written: &str,
    ) -> Result<String, Failure> {
        if annotation.arguments.is_empty() {
            Ok(String::new())
        } else {
            RouteMarker::path_literal(input, annotation, &format!("@{written}"))
        }
    }

    fn missing_mapping(input: &Input<'_>, declaration: &Declaration) -> Failure {
        if super::access_marker::AccessMarker::only_access(declaration) {
            input.failure(
                declaration.start,
                super::access_marker::AccessMarker::METHOD_ADVICE,
            )
        } else {
            input.failure(
                declaration.start,
                "@ResponseStatus needs a route annotation on the same method, such as @GetRoute or @PostRoute",
            )
        }
    }

    fn status_code(input: &Input<'_>, annotation: &Annotation) -> Result<u16, Failure> {
        annotation
            .arguments
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|code| (200..=299).contains(code))
            .ok_or_else(|| {
                input.failure(
                    annotation.span.start,
                    "@ResponseStatus takes one success status number from 200 to 299, for example @ResponseStatus(201). It sets the status of a normal answer",
                )
            })
    }

    fn check_verb_prefix(
        &self,
        input: &Input<'_>,
        start: usize,
        name: &str,
    ) -> Result<(), Failure> {
        let verb = self.written.trim_end_matches("Route").to_ascii_lowercase();
        RouteMarker::require(Self::starts_with_verb(name, &verb), || {
            let example = Self::verb_example(input.extension, &verb);
            input.failure(
                start,
                format!(
                    "@{} means this method answers {} requests, so its name must start with '{verb}', for example {example}. Rename {name} or change the annotation",
                    self.written, self.verb
                ),
            )
        })
    }

    fn verb_example(extension: &str, verb: &str) -> String {
        match extension {
            "py" | "rs" | "rb" => format!("{verb}_items"),
            "cs" | "go" => format!("{}Items", RouteMarker::capitalized(verb)),
            _ => format!("{verb}Items"),
        }
    }

    fn starts_with_verb(name: &str, verb: &str) -> bool {
        let prefix_matches = name
            .get(..verb.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(verb));
        prefix_matches
            && name[verb.len()..]
                .chars()
                .next()
                .is_none_or(Self::starts_next_word)
    }

    fn starts_next_word(next: char) -> bool {
        [
            next == '_',
            next.is_ascii_uppercase(),
            next.is_ascii_digit(),
        ]
        .contains(&true)
    }

    fn refuse_unmarked_stream(
        &self,
        input: &Input<'_>,
        start: usize,
        method: &str,
        generator_signature: bool,
        body: &Range<usize>,
    ) -> Result<(), Failure> {
        let generator = super::generator_body(input, body, start) || generator_signature;
        let unmarked = [generator, !self.stream] == [true, true];
        RouteMarker::require(!unmarked, || {
            input.failure(
                start,
                format!(
                    "{method} produces many values with yield, so it answers with a stream. {}",
                    RouteMarker::STREAM_ADVICE
                ),
            )
        })
    }

    fn check_parameters(
        &self,
        input: &Input<'_>,
        start: usize,
        parameters: &[RouteParameter],
        pattern: &str,
    ) -> Result<(), Failure> {
        parameters
            .iter()
            .try_for_each(|parameter| Self::check_parameter(input, start, parameter, pattern))?;
        let requests = Self::count_from(parameters, ParameterSource::Request);
        let bodies = Self::count_from(parameters, ParameterSource::Body);
        RouteMarker::require(requests <= 1, || {
            input.failure(
                start,
                "Mark each route parameter with @FromPath, @FromQuery, @FromBody, @FromHeader or @FromCookie. Only one parameter may be left unmarked, and it receives the whole request",
            )
        })?;
        RouteMarker::require(bodies <= 1, || {
            input.failure(
                start,
                "A request has one body, so only one parameter may use @FromBody",
            )
        })?;
        let get_body = [bodies == 1, self.verb == "GET"] == [true, true];
        RouteMarker::require(!get_body, || {
            input.failure(
                start,
                "@GetRoute means this method answers GET requests, and a GET request has no body. Remove @FromBody or use @PostRoute",
            )
        })
    }

    fn count_from(parameters: &[RouteParameter], source: ParameterSource) -> usize {
        parameters
            .iter()
            .filter(|parameter| parameter.source == source)
            .count()
    }

    fn check_parameter(
        input: &Input<'_>,
        start: usize,
        parameter: &RouteParameter,
        pattern: &str,
    ) -> Result<(), Failure> {
        let php_request = [
            parameter.source == ParameterSource::Request,
            input.extension == "php",
        ] == [true, true];
        RouteMarker::require(!php_request, || {
            input.failure(start, RouteMarker::PHP_REQUEST_ADVICE)
        })?;
        if parameter.source == ParameterSource::Path {
            Self::check_path_variable(input, start, parameter, pattern)
        } else {
            Ok(())
        }
    }

    fn check_path_variable(
        input: &Input<'_>,
        start: usize,
        parameter: &RouteParameter,
        pattern: &str,
    ) -> Result<(), Failure> {
        let wanted = format!("_{}", parameter.name);
        let in_path = pattern.split('/').any(|segment| segment == wanted);
        RouteMarker::require(in_path, || {
            input.failure(
                start,
                format!(
                    "@FromPath means the value comes from the route path, but '{}' has no {{{}}} part. Add {{{}}} to the path or use another marker",
                    RoutePath::template(pattern),
                    parameter.name,
                    parameter.name
                ),
            )
        })
    }
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
            "py" => Self::after_marker(input, after, "->", &[":"]),
            "rs" => Self::after_marker(input, after, "->", &["{", "where"]),
            "go" => Self::until(&input.source[after..], &["{"])
                .trim()
                .to_owned(),
            _ => Self::after_marker(input, after, ":", &["{", "=>", "="]),
        }
    }

    fn documented(
        extension: &str,
        documentation: &str,
        parameters: &mut [RouteParameter],
        written: String,
    ) -> String {
        match extension {
            "rb" => super::ruby::RubyMethod::read_types(documentation, parameters),
            "php" => crate::route_types::PhpDoc::read_types(documentation, parameters, written),
            _ => written,
        }
    }

    fn before_name(input: &Input<'_>, start: usize, opening: usize, name: &str) -> String {
        let prefix = input.source[start..opening].trim_end();
        let prefix = prefix.strip_suffix(name).unwrap_or(prefix);
        let mut words: Vec<&str> = prefix.split_whitespace().collect();
        words.retain(|word| !Self::MODIFIERS.contains(word));
        words.join(" ")
    }

    fn after_marker(input: &Input<'_>, after: usize, marker: &str, stops: &[&str]) -> String {
        input.source[after..]
            .trim_start()
            .strip_prefix(marker)
            .map(|value| Self::until(value, stops))
            .unwrap_or_default()
            .trim()
            .to_owned()
    }

    fn until<'a>(text: &'a str, stops: &[&str]) -> &'a str {
        let end = stops
            .iter()
            .filter_map(|stop| text.find(stop))
            .min()
            .unwrap_or(text.len());
        &text[..end]
    }
}

struct ParameterMarker {
    source: ParameterSource,
    name: Option<String>,
    length: usize,
}

impl ParameterMarker {
    const NAME_PUNCTUATION: [u8; 3] = *b"_-.";
    const MARKER_PUNCTUATION: [u8; 2] = *b"_:";
    const MAX_NAME: usize = 256;

    fn read_marker(input: &Input<'_>, at: usize, piece: &str) -> Result<Option<Self>, Failure> {
        let prefix = Self::prefix(input.extension);
        piece.strip_prefix(prefix).map_or(Ok(None), |rest| {
            Self::read_known_marker(input, at, piece, prefix, rest)
        })
    }

    fn prefix(extension: &str) -> &'static str {
        match extension {
            "rs" | "php" | "rb" => "#[",
            "cs" => "[",
            _ => "@",
        }
    }

    fn read_known_marker(
        input: &Input<'_>,
        at: usize,
        piece: &str,
        prefix: &str,
        rest: &str,
    ) -> Result<Option<Self>, Failure> {
        let length = rest
            .bytes()
            .take_while(|byte| Self::is_marker_byte(*byte))
            .count();
        let name = &rest[..length];
        RouteMarker::source(name).map_or_else(
            || RouteMarker::refuse_client_marker(input, at, name),
            |found| {
                Self::read_source_marker(input, at, piece, prefix, prefix.len() + length, found)
                    .map(Some)
            },
        )
    }

    fn is_marker_byte(byte: u8) -> bool {
        let punctuation = Self::MARKER_PUNCTUATION.contains(&byte);
        [byte.is_ascii_alphanumeric(), punctuation].contains(&true)
    }

    fn read_source_marker(
        input: &Input<'_>,
        at: usize,
        piece: &str,
        prefix: &str,
        name_end: usize,
        (source, written): (ParameterSource, &'static str),
    ) -> Result<Self, Failure> {
        RouteMarker::require(input.extension != "py", || {
            input.failure(
                at,
                crate::annotation_rules::AnnotationRules::python_parameter(written),
            )
        })?;
        let (name, end) = Self::marker_arguments(input, at, piece, name_end, written)?;
        let length = Self::bracket_end(input, at, piece, prefix, end, written)?;
        Ok(Self {
            source,
            name,
            length,
        })
    }

    fn marker_arguments(
        input: &Input<'_>,
        at: usize,
        piece: &str,
        end: usize,
        written: &str,
    ) -> Result<(Option<String>, usize), Failure> {
        if piece[end..].starts_with('(') {
            Self::marker_argument(input, at, piece, end, written)
        } else {
            Ok((None, end))
        }
    }

    fn marker_argument(
        input: &Input<'_>,
        at: usize,
        piece: &str,
        end: usize,
        written: &str,
    ) -> Result<(Option<String>, usize), Failure> {
        let closing = matching(piece, end, b'(', b')')
            .ok_or_else(|| input.failure(at, format!("Close the arguments of @{written}")))?;
        let original = &input.source[at..at + piece.len()];
        let name = Self::marker_name(input, at, &original[end + 1..closing], written)?;
        Ok((Some(name), closing + 1))
    }

    fn bracket_end(
        input: &Input<'_>,
        at: usize,
        piece: &str,
        prefix: &str,
        end: usize,
        written: &str,
    ) -> Result<usize, Failure> {
        if prefix == "@" {
            return Ok(end);
        }
        RouteMarker::require(piece[end..].starts_with(']'), || {
            input.failure(at, format!("Close @{written} with ']'"))
        })?;
        Ok(end + 1)
    }

    fn marker_name(
        input: &Input<'_>,
        at: usize,
        text: &str,
        written: &str,
    ) -> Result<String, Failure> {
        RouteMarker::quoted_inner(text.trim())
            .filter(|inner| Self::is_marker_name(inner))
            .map(str::to_owned)
            .ok_or_else(|| {
                input.failure(
                    at,
                    format!(
                        "@{written} takes one name in quotes, for example @{written}(\"page_size\"). Without a name it uses the parameter name"
                    ),
                )
            })
    }

    fn is_marker_name(inner: &str) -> bool {
        !inner.is_empty() && inner.len() <= Self::MAX_NAME && inner.bytes().all(Self::is_name_byte)
    }

    fn is_name_byte(byte: u8) -> bool {
        let punctuation = Self::NAME_PUNCTUATION.contains(&byte);
        [byte.is_ascii_alphanumeric(), punctuation].contains(&true)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::MethodMarkers;
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
            assert!(MethodMarkers::starts_with_verb(name, "get"), "{name}");
        }
        for name in ["getter", "target", "ge", "post"] {
            assert!(!MethodMarkers::starts_with_verb(name, "get"), "{name}");
        }
    }
}
