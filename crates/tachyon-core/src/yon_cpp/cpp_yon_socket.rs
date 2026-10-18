use super::{CppYonAccess, CppYonController, CppYonPath};
use crate::Failure;
use crate::routing::RoutePath;
use crate::tac_cpp::{CppAttribute, CppClass, CppFailure, CppMember, CppTypeText};
use crate::yon::plan::{ParameterSource, Route, RouteParameter, SocketMethod, SocketMethods};
use std::path::Path;

pub(crate) struct CppYonSocket<'a> {
    path: &'a Path,
    class: &'a CppClass,
}

impl<'a> CppYonSocket<'a> {
    pub(crate) const MARKER: &'static str = "SocketRoute";
    const EVENTS: [(&'static str, usize); 4] = [
        ("OnOpen", 1),
        ("OnMessage", 2),
        ("OnClose", 1),
        ("OnError", 2),
    ];
    const MESSAGE: &'static str = "OnMessage";
    const VOID: &'static str = "void";
    const VARIABLE: &'static str =
        "A [[SocketRoute]] path has no {variables}. Send the value in a message instead";
    const NO_MESSAGE: &'static str = "A [[SocketRoute]] class needs one [[OnMessage]] method. [[OnMessage]] runs the method for each message that the browser sends, for example `void onMessage(SocketSession& session, ChatMessage message)`";

    pub(crate) fn marker(class: &CppClass) -> Option<&CppAttribute> {
        class
            .markers
            .iter()
            .find(|marker| marker.name == Self::MARKER)
    }

    pub(crate) fn read(
        path: &Path,
        class: &CppClass,
        marker: &CppAttribute,
    ) -> Result<Route, Failure> {
        let socket = CppYonSocket { path, class };
        let pattern = socket.pattern(marker)?;
        let message = socket
            .event(Self::MESSAGE)?
            .ok_or_else(|| socket.failure(0, Self::NO_MESSAGE))?;
        let parameter = socket.message_parameter(message)?;
        Ok(Route {
            verb: String::from("GET"),
            method: message.name.clone(),
            socket: Some(SocketMethods {
                open: socket.method("OnOpen")?,
                message: message.name.clone(),
                close: socket.method("OnClose")?,
                error: socket.method("OnError")?,
            }),
            pattern: Some(pattern),
            parameters: vec![parameter],
            returns: String::from(Self::VOID),
            access: CppYonAccess::read(&class.markers),
            ..Route::default()
        })
    }

    fn pattern(&self, marker: &CppAttribute) -> Result<String, Failure> {
        let written = CppYonPath::of(self.path, marker)?;
        let plain = (!written.contains('{'))
            .then_some(written)
            .ok_or_else(|| self.failure(marker.at, Self::VARIABLE))?;
        RoutePath::pattern("", &plain).map_err(|message| self.failure(marker.at, &message))
    }

    fn method(&self, name: &str) -> Result<Option<SocketMethod>, Failure> {
        let found = self.event(name)?;
        Ok(found.map(|member| SocketMethod {
            name: member.name.clone(),
            parameters: member.parameters.len(),
        }))
    }

    fn event(&self, name: &str) -> Result<Option<&'a CppMember>, Failure> {
        let marked: Vec<&CppMember> = self
            .class
            .members
            .iter()
            .filter(|member| member.marked(name))
            .collect();
        let once = (marked.len() < 2)
            .then_some(marked.first().copied())
            .ok_or_else(|| {
                self.failure(
                    0,
                    &format!("A [[SocketRoute]] class has at most one [[{name}]] method"),
                )
            })?;
        once.map(|member| self.check_event(name, member))
            .transpose()
    }

    fn check_event(&self, name: &str, member: &'a CppMember) -> Result<&'a CppMember, Failure> {
        let most = Self::EVENTS
            .iter()
            .find(|(event, _)| *event == name)
            .map_or(0, |(_, most)| *most);
        let count = member.parameters.len();
        let counted = [count <= most, count == most][usize::from(name == Self::MESSAGE)];
        let shaped = [
            counted,
            !member.is_static,
            member.plain_type() == Self::VOID,
        ];
        (shaped == [true, true, true])
            .then_some(member)
            .ok_or_else(|| self.failure(member.name_at, &Self::advice(name)))
    }

    fn advice(name: &str) -> String {
        let shape = match name {
            "OnMessage" => "(SocketSession& session, ChatMessage message)",
            "OnError" => {
                "(SocketSession& session, const std::string& error), (SocketSession& session) or ()"
            }
            _ => "(SocketSession& session) or ()",
        };
        format!(
            "A C++ [[{name}]] method is an instance method that returns void and takes {shape}. The session sends messages back and closes the socket, and a thrown exception counts as a failure"
        )
    }

    fn message_parameter(&self, member: &CppMember) -> Result<RouteParameter, Failure> {
        let parameter = &member.parameters[1];
        let type_text = CppTypeText::plain(&parameter.type_text);
        let named = [!parameter.name.is_empty(), !type_text.is_empty()] == [true, true];
        let message = format!(
            "The message parameter of {} needs a name and a model type, so Tachyon can check each message. Write `ChatMessage message`, with the model in server/models/",
            member.name
        );
        named
            .then(|| RouteParameter {
                source: ParameterSource::Body,
                name: parameter.name.clone(),
                code_name: parameter.name.clone(),
                type_text,
                shape: crate::route_types::Shape::Any,
            })
            .ok_or_else(|| self.failure(member.name_at, &message))
    }

    fn failure(&self, at: usize, message: &str) -> Failure {
        let at = [at, self.class.name_at][usize::from(at == 0)];
        CppFailure::coded(CppYonController::CODE, self.path, at, message)
    }
}
