use super::routes::RouteMarker;
use super::{
    Annotation, Declaration, Input, ManagedType, declared_type, method_owner, method_signature,
};
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
    const MESSAGE: &'static str = "onmessage";

    pub(super) fn is_name(name: &str) -> bool {
        [name == Self::CLASS, Self::event(name).is_some()].contains(&true)
    }

    fn event(name: &str) -> Option<(&'static str, &'static str, usize)> {
        Self::EVENTS
            .iter()
            .find(|(marker, _, _)| *marker == name)
            .copied()
    }

    fn require(condition: bool, refusal: impl FnOnce() -> Failure) -> Result<(), Failure> {
        condition.then_some(()).ok_or_else(refusal)
    }

    fn refuse_unavailable(input: &Input<'_>) -> Result<(), Failure> {
        crate::annotation_rules::AnnotationRules::unavailable("SocketRoute", input.extension, "yon")
            .map_or(Ok(()), |refusal| Err(input.failure(0, refusal)))
    }

    fn socket_pattern(input: &Input<'_>, declarations: &[Declaration]) -> Result<String, Failure> {
        declarations
            .iter()
            .flat_map(|declaration| &declaration.annotations)
            .find(|annotation| annotation.name == Self::CLASS)
            .map_or_else(
                || Ok(String::from("/")),
                |annotation| Self::annotation_pattern(input, annotation),
            )
    }

    fn annotation_pattern(input: &Input<'_>, annotation: &Annotation) -> Result<String, Failure> {
        let path = Self::written_path(input, annotation)?;
        Self::require(!path.contains('{'), || {
            input.failure(
                annotation.span.start,
                "A @SocketRoute path has no {variables}. Send the value in a message instead",
            )
        })?;
        RoutePath::pattern("", &path)
            .map_err(|message| input.failure(annotation.span.start, message))
    }

    fn written_path(input: &Input<'_>, annotation: &Annotation) -> Result<String, Failure> {
        if annotation.arguments.is_empty() {
            Ok(String::new())
        } else {
            RouteMarker::path_literal(input, annotation, "@SocketRoute")
        }
    }

    fn event_marker(
        input: &Input<'_>,
        declaration: &Declaration,
    ) -> Result<(&'static str, &'static str, usize), Failure> {
        declaration
            .annotations
            .iter()
            .try_fold(None, |found, annotation| {
                Self::only_event(input, annotation, found)
            })?
            .ok_or_else(|| input.failure(declaration.start, "A socket method needs a socket event"))
    }

    fn only_event(
        input: &Input<'_>,
        annotation: &Annotation,
        found: Option<(&'static str, &'static str, usize)>,
    ) -> Result<Option<(&'static str, &'static str, usize)>, Failure> {
        let event = Self::event(&annotation.name).ok_or_else(|| {
            input.failure(
                annotation.span.start,
                "A @SocketRoute class answers socket events only. Mark its methods with @OnOpen, @OnMessage, @OnClose or @OnError",
            )
        })?;
        Self::require(found.is_none(), || {
            input.failure(
                annotation.span.start,
                "A method answers one socket event. Keep one of @OnOpen, @OnMessage, @OnClose or @OnError",
            )
        })?;
        Ok(Some(event))
    }

    fn takes_parameters(marker: &str, most: usize, count: usize) -> bool {
        match marker {
            Self::MESSAGE => count == most,
            _ => count <= most,
        }
    }

    fn message_parameter(
        input: &Input<'_>,
        start: usize,
        piece: &str,
        documentation: &str,
    ) -> Result<RouteParameter, Failure> {
        let (code_name, written_type) = RouteMarker::name_and_type(input.extension, piece)
            .ok_or_else(|| input.failure(start, format!("Cannot read the parameter '{piece}'")))?;
        let type_text =
            Self::message_type(input.extension, written_type, documentation, &code_name);
        let documented =
            !crate::route_types::RouteTypes::documented_type(documentation, "@param", &code_name)
                .is_empty();
        Self::require([!type_text.is_empty(), documented].contains(&true), || {
            input.failure(
                start,
                format!(
                    "The message parameter '{code_name}' needs a model type, so Tachyon can check each message. Write `{code_name}: ChatMessage` in TypeScript or `@param {{ChatMessage}} {code_name}` in JavaScript, with the model in server/models/"
                ),
            )
        })?;
        Ok(RouteParameter {
            source: ParameterSource::Body,
            name: code_name.clone(),
            code_name,
            type_text,
            shape: crate::route_types::Shape::Any,
        })
    }

    fn message_type(
        extension: &str,
        written_type: String,
        documentation: &str,
        code_name: &str,
    ) -> String {
        if extension == "rb" {
            crate::ruby_source::Yard::parameter_type(documentation, code_name)
        } else {
            written_type
        }
    }

    fn without_receiver(extension: &str, pieces: Vec<String>) -> Vec<String> {
        let receiver = Self::is_receiver(extension, pieces.first().map(String::as_str));
        pieces.into_iter().skip(usize::from(receiver)).collect()
    }

    fn is_receiver(extension: &str, first: Option<&str>) -> bool {
        match (extension, first) {
            ("py", Some("self")) => true,
            ("rs", Some(first)) => first.ends_with("self"),
            _ => false,
        }
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
}

struct SocketHandlers {
    pattern: String,
    methods: SocketMethods,
    message_route: Option<Route>,
}

impl SocketHandlers {
    fn add(
        &mut self,
        input: &Input<'_>,
        declaration: &Declaration,
        types: &[ManagedType],
        controller: &str,
    ) -> Result<(), Failure> {
        let (marker, written, most) = SocketMarker::event_marker(input, declaration)?;
        let start = declaration.start;
        let owner = method_owner(input, start, types)?;
        SocketMarker::require(owner == controller, || {
            input.failure(
                start,
                format!("@{written} belongs on a method of the @SocketRoute class, not on {owner}"),
            )
        })?;
        let signature = method_signature(input, start)?;
        let list = RouteMarker::parameter_list(input, start)?;
        let pieces = super::parameters(input, &input.code[list.clone()], start)?;
        let pieces = SocketMarker::without_receiver(input.extension, pieces);
        SocketMarker::require(
            SocketMarker::takes_parameters(marker, most, pieces.len()),
            || input.failure(start, SocketMarker::parameter_advice(written)),
        )?;
        self.store(
            input,
            declaration,
            (marker, written),
            signature.name,
            &pieces,
        )
    }

    fn store(
        &mut self,
        input: &Input<'_>,
        declaration: &Declaration,
        (marker, written): (&str, &str),
        method: String,
        pieces: &[String],
    ) -> Result<(), Failure> {
        if marker == SocketMarker::MESSAGE {
            self.store_message_route(input, declaration, method, pieces)
        } else {
            self.store_event_method(input, declaration.start, (marker, written), method, pieces)
        }
    }

    fn store_message_route(
        &mut self,
        input: &Input<'_>,
        declaration: &Declaration,
        method: String,
        pieces: &[String],
    ) -> Result<(), Failure> {
        let start = declaration.start;
        SocketMarker::require(self.message_route.is_none(), || {
            input.failure(start, SocketMarker::twice("OnMessage"))
        })?;
        let documentation = RouteMarker::documentation_before(input, declaration);
        let message = SocketMarker::message_parameter(input, start, &pieces[1], &documentation)?;
        self.methods.message.clone_from(&method);
        self.message_route = Some(Route {
            verb: String::from("GET"),
            method,
            socket: Some(SocketMethods::default()),
            pattern: Some(self.pattern.clone()),
            parameters: vec![message],
            documentation,
            ..Route::default()
        });
        Ok(())
    }

    fn store_event_method(
        &mut self,
        input: &Input<'_>,
        start: usize,
        (marker, written): (&str, &str),
        method: String,
        pieces: &[String],
    ) -> Result<(), Failure> {
        let slot = Self::event_slot(&mut self.methods, marker);
        SocketMarker::require(slot.is_none(), || {
            input.failure(start, SocketMarker::twice(written))
        })?;
        *slot = Some(SocketMethod {
            name: method,
            parameters: pieces.len(),
        });
        Ok(())
    }

    fn event_slot<'a>(
        methods: &'a mut SocketMethods,
        marker: &str,
    ) -> &'a mut Option<SocketMethod> {
        match marker {
            "onopen" => &mut methods.open,
            "onclose" => &mut methods.close,
            _ => &mut methods.error,
        }
    }

    fn into_route(self, input: &Input<'_>) -> Result<Route, Failure> {
        let Some(mut route) = self.message_route else {
            return Err(input.failure(
                0,
                "A @SocketRoute class needs one @OnMessage method. @OnMessage runs the method for each message that the browser sends, for example onMessage(session, message)",
            ));
        };
        route.socket = Some(self.methods);
        Ok(route)
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
    SocketMarker::refuse_unavailable(input)?;
    let mut handlers = SocketHandlers {
        pattern: SocketMarker::socket_pattern(input, declarations)?,
        methods: SocketMethods::default(),
        message_route: None,
    };
    declarations
        .iter()
        .filter(|declaration| declared_type(input, declaration).is_none())
        .try_for_each(|declaration| handlers.add(input, declaration, types, controller))?;
    handlers.into_route(input)
}
