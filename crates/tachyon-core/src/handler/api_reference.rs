use crate::{Failure, RouteGraph};
use std::path::PathBuf;

struct ApiReferenceAssets;

impl ApiReferenceAssets {
    const FILES: [(&str, &str); 3] = [
        ("index.html", include_str!("../assets/api-docs/index.html")),
        ("viewer.css", include_str!("../assets/api-docs/viewer.css")),
        ("viewer.js", include_str!("../assets/api-docs/viewer.js")),
    ];
}

pub(crate) fn files(graph: &RouteGraph) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
    let mut models = serde_json::Map::new();
    let routes = controller_routes(graph, &mut models);
    if routes.is_empty() {
        return Ok(Vec::new());
    }
    let document = serde_json::json!({
        "contract_version": 1,
        "version": tachyon_contracts::PRODUCT_VERSION,
        "routes": routes,
        "models": models,
    });
    let bytes = serde_json::to_vec_pretty(&document).map_err(|_| {
        Failure::one(crate::failure::diagnostic(
            2005,
            "Cannot encode the declared API reference.",
            None,
            None,
        ))
    })?;
    let mut files = vec![(PathBuf::from("api.json"), bytes)];
    for (name, source) in ApiReferenceAssets::FILES {
        let path = PathBuf::from("api-docs").join(name);
        files.push((path, source.as_bytes().to_vec()));
    }
    Ok(files)
}

fn controller_routes(
    graph: &RouteGraph,
    models: &mut serde_json::Map<String, serde_json::Value>,
) -> Vec<serde_json::Value> {
    let mut routes = Vec::new();
    for route in graph.routes() {
        let mut typed = Vec::new();
        for handler in route.handlers() {
            let source = handler.source();
            for mapping in source.routes() {
                if mapping.pattern.as_deref() == Some(route.route()) {
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
