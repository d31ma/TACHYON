use crate::server::RouteGuard;
use crate::sorted_json::SortedJson;
use crate::yon::route_access::RouteAccess;
use crate::{Failure, RouteGraph};
use serde_json::{Map, Value};
use std::path::PathBuf;

pub(crate) struct ApiReference;

impl ApiReference {
    const MODEL_REFERENCE: &'static str = "#/models/";
    const ASSETS: [(&'static str, &'static str); 3] = [
        ("index.html", include_str!("../assets/api-docs/index.html")),
        ("viewer.css", include_str!("../assets/api-docs/viewer.css")),
        ("viewer.js", include_str!("../assets/api-docs/viewer.js")),
    ];

    pub(crate) fn published_files(graph: &RouteGraph) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        let mut models = Map::new();
        let routes = Self::controller_routes(graph, RouteGuard::allows_anonymous, &mut models);
        if routes.is_empty() {
            return Ok(Vec::new());
        }
        let models = Self::referenced_models(&routes, &models);
        let document = Self::document(&routes, &models);
        let bytes = serde_json::to_vec_pretty(&SortedJson::value(&document)).map_err(|_| {
            Failure::one(crate::failure::diagnostic(
                2005,
                "Cannot encode the declared API reference.",
                None,
                None,
            ))
        })?;
        let mut files = vec![(PathBuf::from("api.json"), bytes)];
        for (name, source) in Self::ASSETS {
            let path = PathBuf::from("api-docs").join(name);
            files.push((path, source.as_bytes().to_vec()));
        }
        Ok(files)
    }

    pub(crate) fn contract(graph: &RouteGraph) -> Value {
        let mut models = Map::new();
        let routes = Self::controller_routes(graph, |_| true, &mut models);
        if routes.is_empty() {
            return serde_json::json!({ "routes": [], "models": {} });
        }
        Self::document(&routes, &models)
    }

    fn document(routes: &[Value], models: &Map<String, Value>) -> Value {
        serde_json::json!({
            "contract_version": 1,
            "version": tachyon_contracts::PRODUCT_VERSION,
            "routes": routes,
            "models": models,
        })
    }

    fn controller_routes(
        graph: &RouteGraph,
        shown: fn(&RouteAccess) -> bool,
        models: &mut Map<String, Value>,
    ) -> Vec<Value> {
        let mut routes = Vec::new();
        for route in graph.routes() {
            let mut typed = Vec::new();
            for handler in route.handlers() {
                let source = handler.source();
                for mapping in source.routes() {
                    if mapping.pattern.as_deref() == Some(route.route()) && shown(&mapping.access) {
                        typed.push((mapping, source.models()));
                    }
                }
                super::RouteSchema::add_models(models, source.models());
            }
            if typed.is_empty() {
                continue;
            }
            routes.push(super::RouteSchema::api_route(
                route.route(),
                &route.parameters(),
                &typed,
            ));
        }
        routes
    }

    fn referenced_models(routes: &[Value], models: &Map<String, Value>) -> Map<String, Value> {
        let mut wanted = Vec::new();
        for route in routes {
            Self::collect_references(route, &mut wanted);
        }
        let mut referenced = Map::new();
        while let Some(name) = wanted.pop() {
            let Some(schema) = models
                .get(&name)
                .filter(|_| !referenced.contains_key(&name))
            else {
                continue;
            };
            Self::collect_references(schema, &mut wanted);
            referenced.insert(name, schema.clone());
        }
        referenced
    }

    fn collect_references(value: &Value, found: &mut Vec<String>) {
        match value {
            Value::Object(fields) => {
                let reference = fields.get("$ref").and_then(Value::as_str);
                let name = reference.and_then(|text| text.strip_prefix(Self::MODEL_REFERENCE));
                found.extend(name.map(String::from));
                fields
                    .values()
                    .for_each(|field| Self::collect_references(field, found));
            }
            Value::Array(items) => items
                .iter()
                .for_each(|item| Self::collect_references(item, found)),
            _ => {}
        }
    }
}
