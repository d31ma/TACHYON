use super::dynamic::SocketCalls;
use super::plan::{Injection, Plan, Provider, Route, Scope, SocketMethods};
use super::route_calls::RouteCalls;
use crate::go_source::GoFile;
use std::fmt::Write as _;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum GoRouteResult {
    Nothing,
    Value,
    Failure,
    ValueAndFailure,
}

impl GoRouteResult {
    pub(crate) fn of(results: &str) -> Result<Self, String> {
        let results = results.trim();
        if results.is_empty() {
            return Ok(Self::Nothing);
        }
        if results == "error" {
            return Ok(Self::Failure);
        }
        let Some(inside) = results
            .strip_prefix('(')
            .and_then(|rest| rest.strip_suffix(')'))
        else {
            return Ok(Self::Value);
        };
        let parts = crate::go_source::split_top_level(inside, b',');
        let last = parts.last().map_or("", |part| part.trim());
        let returns_error = last == "error" || last.ends_with(" error");
        match parts.len() {
            1 => Self::of(inside),
            2 if returns_error => Ok(Self::ValueAndFailure),
            _ => Err(String::from(
                "A Go route returns one value, or a value and an error",
            )),
        }
    }

    fn handler_results(self) -> &'static str {
        match self {
            Self::Nothing | Self::Value => "any",
            Self::Failure | Self::ValueAndFailure => "(any, error)",
        }
    }

    fn handler_body(self, call: &str) -> String {
        match self {
            Self::Nothing => format!("\t{call}\n\treturn nil"),
            Self::Value | Self::ValueAndFailure => format!("\treturn {call}"),
            Self::Failure => format!("\treturn nil, {call}"),
        }
    }
}

pub(crate) struct GoContainer;

impl GoContainer {
    const TYPE: &str = "__YonContainer";

    pub(crate) fn render(
        stripped: &str,
        plan: &Plan,
        dependencies: &[Vec<usize>],
        controller: usize,
    ) -> Result<String, String> {
        let mut text = format!("{}container\npackage main\n", GoFile::MARKER);
        text.push_str(&Self::providers(plan, dependencies));
        text.push_str("\ntype Handler struct{}\n");
        for route in &plan.routes {
            let result = Self::dispatch_result(stripped, &plan.controller, route)?;
            Self::handler_method(&mut text, plan, route, controller, result);
        }
        Ok(text)
    }

    pub(crate) fn socket(
        stripped: &str,
        plan: &Plan,
        dependencies: &[Vec<usize>],
        controller: usize,
        methods: &SocketMethods,
    ) -> Result<String, String> {
        let mut text = format!("{}container\npackage main\n", GoFile::MARKER);
        text.push_str(&Self::providers(plan, dependencies));
        let _ = write!(
            text,
            "\ntype __YonSocketEndpoint struct {{\n\ttarget *{}\n}}\n",
            plan.controller
        );
        for (event, method, values) in [
            ("Open", methods.open.as_ref(), ["session", ""]),
            ("Close", methods.close.as_ref(), ["session", ""]),
            ("Fail", methods.error.as_ref(), ["session", "failure"]),
        ] {
            let parameters = if event == "Fail" {
                "session *SocketSession, failure error"
            } else {
                "session *SocketSession"
            };
            let body = match method {
                Some(method) => {
                    let call = format!(
                        "endpoint.target.{}({})",
                        method.name,
                        SocketCalls::arguments(method, &values)
                    );
                    Self::socket_body(stripped, &plan.controller, &method.name, &call)?
                }
                None => String::from("\treturn nil"),
            };
            let _ = write!(
                text,
                "\nfunc (endpoint __YonSocketEndpoint) {event}({parameters}) error {{\n{body}\n}}\n"
            );
        }
        let kind = SocketCalls::message_type(plan).unwrap_or("any");
        let call = format!("endpoint.target.{}(session, message)", methods.message);
        let body = Self::socket_body(stripped, &plan.controller, &methods.message, &call)?;
        let _ = write!(
            text,
            "\nfunc (endpoint __YonSocketEndpoint) Message(session *SocketSession, data string) error {{\n\tvar message {kind}\n\tif failure := yonSocketRead(data, &message); failure != nil {{\n\t\treturn failure\n\t}}\n{body}\n}}\n"
        );
        let _ = write!(
            text,
            "\ntype Handler struct{{}}\n\nfunc (Handler) GET(request *yonRequest) any {{\n\tyonSocketRun(__YonSocketEndpoint{{target: (&{}{{}}).resolve_{controller}()}})\n\treturn nil\n}}\n",
            Self::TYPE
        );
        Ok(text)
    }

    fn socket_body(
        stripped: &str,
        controller: &str,
        method: &str,
        call: &str,
    ) -> Result<String, String> {
        Ok(match Self::route_result(stripped, controller, method)? {
            GoRouteResult::Nothing | GoRouteResult::Value => format!("\t{call}\n\treturn nil"),
            GoRouteResult::Failure => format!("\treturn {call}"),
            GoRouteResult::ValueAndFailure => format!("\t_, failure := {call}\n\treturn failure"),
        })
    }

    pub(crate) fn route_result(
        stripped: &str,
        controller: &str,
        method: &str,
    ) -> Result<GoRouteResult, String> {
        let code = crate::annotation_layout::masked_source(Path::new("yon.go"), stripped)
            .map_err(|error| error.to_string())?;
        for declared in crate::go_source::methods(&code) {
            if declared.receiver_type == controller && declared.name == method {
                return GoRouteResult::of(&code[declared.results]);
            }
        }
        Err(format!("Go route {controller}.{method} is not declared."))
    }

    pub(crate) fn dispatch_result(
        stripped: &str,
        controller: &str,
        route: &Route,
    ) -> Result<GoRouteResult, String> {
        if route.dispatcher {
            return Ok(GoRouteResult::ValueAndFailure);
        }
        Self::route_result(stripped, controller, &route.method)
    }

    pub(crate) fn providers(plan: &Plan, dependencies: &[Vec<usize>]) -> String {
        let mut text = format!("\ntype {} struct {{\n", Self::TYPE);
        for (index, provider) in plan.providers.iter().enumerate() {
            if provider.scope != Scope::Transient {
                let _ = writeln!(text, "\tp{index} *{}", provider.name);
            }
        }
        text.push_str("}\n");
        for (index, provider) in plan.providers.iter().enumerate() {
            Self::resolver(&mut text, index, provider, &dependencies[index]);
        }
        text
    }

    fn resolver(text: &mut String, index: usize, provider: &Provider, dependencies: &[usize]) {
        let cached = provider.scope != Scope::Transient;
        let _ = writeln!(
            text,
            "\nfunc (container *{}) resolve_{index}() *{} {{",
            Self::TYPE,
            provider.name
        );
        if cached {
            let _ = writeln!(
                text,
                "\tif container.p{index} != nil {{\n\t\treturn container.p{index}\n\t}}"
            );
        }
        let _ = writeln!(text, "\tvalue := &{}{{}}", provider.name);
        for (injection, dependency) in provider.injections.iter().zip(dependencies) {
            let _ = writeln!(text, "\t{}", Self::assignment(injection, *dependency));
        }
        if cached {
            let _ = writeln!(text, "\tcontainer.p{index} = value");
        }
        text.push_str("\treturn value\n}\n");
    }

    fn assignment(injection: &Injection, dependency: usize) -> String {
        let dereference = if injection.pointer { "" } else { "*" };
        format!(
            "value.{} = {dereference}container.resolve_{dependency}()",
            injection.field
        )
    }

    fn handler_method(
        text: &mut String,
        plan: &Plan,
        route: &Route,
        controller: usize,
        result: GoRouteResult,
    ) {
        let argument = match (route.takes_request, route.dispatcher) {
            (false, _) => "",
            (true, true) => "request",
            (true, false) => "yonNativeRequest(request)",
        };
        let mut errors = Vec::new();
        for (index, _) in RouteCalls::error_owners(plan) {
            errors.push(format!("container.resolve_{index}()"));
        }
        let mut setup = String::new();
        let mut container = format!("(&{}{{}})", Self::TYPE);
        if !errors.is_empty() {
            setup = format!("\tcontainer := &{}{{}}\n", Self::TYPE);
            container = String::from("container");
        }
        let call = RouteCalls::invocation_errors(
            route,
            &format!("{container}.resolve_{controller}()"),
            ".",
            argument,
            &errors,
        );
        let _ = writeln!(
            text,
            "\nfunc (Handler) {}(request *yonRequest) {} {{\n{setup}{}\n}}",
            route.verb,
            result.handler_results(),
            result.handler_body(&call)
        );
    }
}
