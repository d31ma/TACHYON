use super::routes::{documentation_before, name_and_type, parameter_list, path_literal};
use super::{Declaration, Input, ManagedType, declared_type, method_owner, method_signature};
use crate::Failure;
use crate::routing::RoutePath;
use crate::yon::plan::{ParameterSource, Route, RouteParameter, SocketMethod, SocketMethods};

pub(super) struct SocketMarker;

impl SocketMarker {
    pub(super) const CLASS: &'static str = "socketroute";
    const EVENTS: [(&'static str, &'static str, usize); 4] = [
        ("onopen", "OnOpen", 1),
        ("onmessage", "OnMessage", 2),
        ("onclose", "OnClose", 1),
        ("onerror", "OnError", 2),
    ];

    pub(super) fn is_name(name: &str) -> bool {
        name == Self::CLASS || Self::EVENTS.iter().any(|(marker, _, _)| *marker == name)
    }

    fn event(name: &str) -> Option<(&'static str, &'static str, usize)> {
        Self::EVENTS
            .iter()
            .find(|(marker, _, _)| *marker == name)
            .copied()
    }
}

pub(super) fn uses_socket_route(declarations: &[Declaration]) -> bool {
    declarations.iter().any(|declaration| {
        declaration
            .annotations
            .iter()
            .any(|annotation| annotation.name == SocketMarker::CLASS)
    })
}

pub(super) fn mapped(
    input: &Input<'_>,
    declarations: &[Declaration],
    types: &[ManagedType],
    controller: &str,
) -> Result<Route, Failure> {
    if let Some(refusal) =
        crate::annotation_rules::AnnotationRules::unavailable("SocketRoute", input.extension, "yon")
    {
        return Err(input.failure(0, refusal));
    }
    let pattern = socket_pattern(input, declarations)?;
    let mut methods = SocketMethods::default();
    let mut route = None;
    for declaration in declarations {
        if declared_type(input, declaration).is_some() {
            continue;
        }
        let (marker, written, most) = event_marker(input, declaration)?;
        let start = declaration.start;
        let owner = method_owner(input, start, types)?;
        if owner != controller {
            return Err(input.failure(
                start,
                format!("@{written} belongs on a method of the @SocketRoute class, not on {owner}"),
            ));
        }
        let signature = method_signature(input, start)?;
        let list = parameter_list(input, start)?;
        let pieces = super::parameters(input, &input.code[list.clone()], start)?;
        let pieces = without_receiver(input.extension, pieces);
        if pieces.len() > most || (marker == "onmessage" && pieces.len() != most) {
            return Err(input.failure(start, parameter_advice(written)));
        }
        if marker == "onmessage" {
            if route.is_some() {
                return Err(input.failure(start, twice("OnMessage")));
            }
            let documentation = documentation_before(input, declaration);
            let message = message_parameter(input, start, &pieces[1], &documentation)?;
            methods.message.clone_from(&signature.name);
            route = Some(Route {
                verb: String::from("GET"),
                method: signature.name,
                socket: Some(SocketMethods::default()),
                pattern: Some(pattern.clone()),
                parameters: vec![message],
                documentation,
                ..Route::default()
            });
            continue;
        }
        let slot = match marker {
            "onopen" => &mut methods.open,
            "onclose" => &mut methods.close,
            _ => &mut methods.error,
        };
        if slot.is_some() {
            return Err(input.failure(start, twice(written)));
        }
        *slot = Some(SocketMethod {
            name: signature.name,
            parameters: pieces.len(),
        });
    }
    let Some(mut route) = route else {
        return Err(input.failure(
            0,
            "A @SocketRoute class needs one @OnMessage method. @OnMessage runs the method for each message that the browser sends, for example onMessage(session, message)",
        ));
    };
    route.socket = Some(methods);
    Ok(route)
}

fn socket_pattern(input: &Input<'_>, declarations: &[Declaration]) -> Result<String, Failure> {
    for declaration in declarations {
        for annotation in &declaration.annotations {
            if annotation.name != SocketMarker::CLASS {
                continue;
            }
            let path = if annotation.arguments.is_empty() {
                String::new()
            } else {
                path_literal(input, annotation, "@SocketRoute")?
            };
            if path.contains('{') {
                return Err(input.failure(
                    annotation.span.start,
                    "A @SocketRoute path has no {variables}. Send the value in a message instead",
                ));
            }
            return RoutePath::pattern("", &path)
                .map_err(|message| input.failure(annotation.span.start, message));
        }
    }
    Ok(String::from("/"))
}

fn event_marker(
    input: &Input<'_>,
    declaration: &Declaration,
) -> Result<(&'static str, &'static str, usize), Failure> {
    let mut found = None;
    for annotation in &declaration.annotations {
        let Some(event) = SocketMarker::event(&annotation.name) else {
            return Err(input.failure(
                annotation.span.start,
                "A @SocketRoute class answers socket events only. Mark its methods with @OnOpen, @OnMessage, @OnClose or @OnError",
            ));
        };
        if found.is_some() {
            return Err(input.failure(
                annotation.span.start,
                "A method answers one socket event. Keep one of @OnOpen, @OnMessage, @OnClose or @OnError",
            ));
        }
        found = Some(event);
    }
    found.ok_or_else(|| input.failure(declaration.start, "A socket method needs a socket event"))
}

fn message_parameter(
    input: &Input<'_>,
    start: usize,
    piece: &str,
    documentation: &str,
) -> Result<RouteParameter, Failure> {
    let (code_name, mut type_text) = name_and_type(input.extension, piece)
        .ok_or_else(|| input.failure(start, format!("Cannot read the parameter '{piece}'")))?;
    if input.extension == "rb" {
        type_text = crate::ruby_source::Yard::parameter_type(documentation, &code_name);
    }
    let documented =
        !crate::route_types::RouteTypes::documented_type(documentation, "@param", &code_name)
            .is_empty();
    if type_text.is_empty() && !documented {
        return Err(input.failure(
            start,
            format!(
                "The message parameter '{code_name}' needs a model type, so Tachyon can check each message. Write `{code_name}: ChatMessage` in TypeScript or `@param {{ChatMessage}} {code_name}` in JavaScript, with the model in server/models/"
            ),
        ));
    }
    Ok(RouteParameter {
        source: ParameterSource::Body,
        name: code_name.clone(),
        code_name,
        type_text,
        shape: crate::route_types::Shape::Any,
    })
}

fn without_receiver(extension: &str, mut pieces: Vec<String>) -> Vec<String> {
    let receiver = match (extension, pieces.first().map(String::as_str)) {
        ("py", Some("self")) => true,
        ("rs", Some(first)) => first.ends_with("self"),
        _ => false,
    };
    if receiver {
        pieces.remove(0);
    }
    pieces
}

fn parameter_advice(written: &str) -> String {
    let shape = match written {
        "OnMessage" => "(session, message)",
        "OnError" => "(session, error), (session) or ()",
        _ => "(session) or ()",
    };
    format!(
        "An @{written} method takes {shape}. The session sends messages back and closes the socket"
    )
}

fn twice(written: &str) -> String {
    format!("A @SocketRoute class has at most one @{written} method")
}
