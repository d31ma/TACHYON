use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Plan {
    pub providers: Vec<Provider>,
    pub controller: String,
    pub routes: Vec<Route>,
    pub errors: Vec<ErrorMethod>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct SocketMethods {
    pub open: Option<SocketMethod>,
    pub message: String,
    pub close: Option<SocketMethod>,
    pub error: Option<SocketMethod>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct SocketMethod {
    pub name: String,
    pub parameters: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ErrorMethod {
    pub owner: String,
    pub method: String,
    pub error_type: String,
    pub takes_request: bool,
    pub status: u16,
    pub returns: String,
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ErrorResponse {
    pub error_type: String,
    pub status: u16,
    pub returns: String,
    #[serde(skip)]
    pub documentation: String,
    pub return_shape: crate::route_types::Shape,
}

impl From<&ErrorMethod> for ErrorResponse {
    fn from(method: &ErrorMethod) -> Self {
        Self {
            error_type: method.error_type.clone(),
            status: method.status,
            returns: method.returns.clone(),
            documentation: method.documentation.clone(),
            return_shape: crate::route_types::Shape::Any,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Provider {
    pub name: String,
    pub role: Role,
    pub scope: Scope,
    pub dependencies: Vec<String>,
    pub injections: Vec<Injection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Injection {
    pub field: String,
    pub pointer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Role {
    Controller,
    Service,
    Repository,
    Client,
    Component,
    ErrorHandler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Scope {
    Singleton,
    Request,
    Transient,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct Route {
    pub verb: String,
    pub method: String,
    pub streaming: bool,
    pub socket: Option<SocketMethods>,
    pub takes_request: bool,
    pub pattern: Option<String>,
    pub status: Option<u16>,
    pub parameters: Vec<RouteParameter>,
    pub returns: String,
    #[serde(skip)]
    pub documentation: String,
    pub return_shape: crate::route_types::Shape,
    pub error_responses: Vec<ErrorResponse>,
    #[serde(skip)]
    pub dispatcher: bool,
    #[serde(skip)]
    pub access: super::route_access::RouteAccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RouteParameter {
    pub source: ParameterSource,
    pub name: String,
    pub code_name: String,
    pub type_text: String,
    pub shape: crate::route_types::Shape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ParameterSource {
    Request,
    Path,
    Query,
    Body,
    Header,
    Cookie,
}
