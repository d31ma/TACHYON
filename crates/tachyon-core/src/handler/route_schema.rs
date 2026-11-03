use crate::route_types::{ModelTable, Shape};
use crate::routing::RoutePath;
use crate::yon::plan::{ParameterSource, Route};
use serde_json::{Map, Value, json};

pub(crate) struct RouteSchema;

impl RouteSchema {
    pub(crate) fn for_path(pattern: &str, routes: &[(&Route, &ModelTable)]) -> Value {
        let mut methods = Map::new();
        let mut models = Map::new();
        for (route, table) in routes {
            methods.insert(route.verb.clone(), Self::method(route));
            Self::add_models(&mut models, table);
        }
        json!({
            "path": RoutePath::template(pattern),
            "methods": methods,
            "models": models,
        })
    }

    pub(crate) fn api_route(
        pattern: &str,
        parameters: &[String],
        routes: &[(&Route, &ModelTable)],
    ) -> Value {
        let mut methods = Map::new();
        for (route, _) in routes {
            methods.insert(route.verb.clone(), Self::method(route));
        }
        json!({
            "route": pattern,
            "path": RoutePath::template(pattern),
            "parameters": parameters,
            "methods": methods,
        })
    }

    pub(crate) fn add_models(models: &mut Map<String, Value>, table: &ModelTable) {
        if let Value::Object(schemas) = table.model_schemas() {
            for (name, schema) in schemas {
                models.insert(name, schema);
            }
        }
    }

    fn method(route: &Route) -> Value {
        let mut request = Map::new();
        for parameter in &route.parameters {
            let Some(place) = Self::place(parameter.source) else {
                continue;
            };
            let schema = ModelTable::schema(&parameter.shape);
            if parameter.source == ParameterSource::Body {
                request.insert(String::from("body"), schema);
                continue;
            }
            let key = if parameter.shape.is_optional() {
                format!("{}?", parameter.name)
            } else {
                parameter.name.clone()
            };
            let entry = request
                .entry(place)
                .or_insert_with(|| Value::Object(Map::new()));
            if let Value::Object(fields) = entry {
                fields.insert(key, schema);
            }
        }
        let mut method = Map::new();
        method.insert(String::from("request"), Value::Object(request));
        method.insert(String::from("status"), json!(route.status.unwrap_or(200)));
        if route.return_shape != Shape::Any {
            method.insert(String::from("ok"), ModelTable::schema(&route.return_shape));
        }
        if !route.error_responses.is_empty() {
            let errors = route
                .error_responses
                .iter()
                .map(|error| {
                    json!({
                        "type": error.error_type,
                        "status": error.status,
                        "schema": ModelTable::schema(&error.return_shape),
                    })
                })
                .collect();
            method.insert(String::from("errors"), Value::Array(errors));
        }
        if route.socket.is_some() {
            method.insert(String::from("socket"), Value::Bool(true));
        }
        if route.streaming {
            method.insert(String::from("stream"), Value::Bool(true));
            method.insert(String::from("produces"), json!("text/event-stream"));
        }
        Value::Object(method)
    }

    const fn place(source: ParameterSource) -> Option<&'static str> {
        match source {
            ParameterSource::Path => Some("parameters"),
            ParameterSource::Query => Some("query"),
            ParameterSource::Header => Some("headers"),
            ParameterSource::Cookie => Some("cookies"),
            ParameterSource::Body => Some("body"),
            ParameterSource::Request => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RouteSchema;
    use crate::route_types::{ModelTable, Shape};
    use crate::yon::plan::Route;

    fn route(streaming: bool) -> Route {
        Route {
            verb: String::from("GET"),
            method: String::from("streamTicks"),
            streaming,
            socket: None,
            takes_request: false,
            pattern: Some(String::from("/ticks")),
            status: None,
            parameters: Vec::new(),
            returns: String::new(),
            documentation: String::new(),
            return_shape: Shape::Any,
            error_responses: Vec::new(),
            dispatcher: false,
            access: crate::yon::route_access::RouteAccess::Public,
        }
    }

    #[test]
    fn a_stream_route_is_described_as_an_event_stream() {
        let table = ModelTable::default();
        let stream = route(true);
        let schema = RouteSchema::api_route("/ticks", &[], &[(&stream, &table)]);
        assert_eq!(schema["methods"]["GET"]["produces"], "text/event-stream");
        assert_eq!(schema["methods"]["GET"]["stream"], true);
        let plain = route(false);
        let schema = RouteSchema::api_route("/ticks", &[], &[(&plain, &table)]);
        assert!(schema["methods"]["GET"].get("produces").is_none());
    }

    #[test]
    fn a_socket_route_is_marked_as_a_socket() {
        let table = ModelTable::default();
        let mut socket = route(false);
        socket.socket = Some(crate::yon::plan::SocketMethods::default());
        let schema = RouteSchema::api_route("/chat", &[], &[(&socket, &table)]);
        assert_eq!(schema["methods"]["GET"]["socket"], true);
        let plain = route(false);
        let schema = RouteSchema::api_route("/chat", &[], &[(&plain, &table)]);
        assert!(schema["methods"]["GET"].get("socket").is_none());
    }
}
