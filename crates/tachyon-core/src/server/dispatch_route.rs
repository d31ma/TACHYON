use crate::Project;
use crate::handler::HandlerSource;
use crate::project::RouteNode;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(in crate::server) struct DispatchRoute {
    pub(in crate::server) route: String,
    pub(in crate::server) handler: HandlerSource,
    pub(in crate::server) mapping: Arc<crate::yon::plan::Route>,
}

impl DispatchRoute {
    pub(in crate::server) fn discover(project: &Project) -> (Vec<Self>, Vec<String>) {
        let page_routes = project
            .route_graph()
            .routes()
            .iter()
            .filter(|route| route.source_path().is_some())
            .map(|route| String::from(route.route()))
            .collect();
        let routes = project
            .route_graph()
            .routes()
            .iter()
            .flat_map(Self::from_route_node)
            .collect();
        (routes, page_routes)
    }

    fn from_route_node(route: &RouteNode) -> Vec<Self> {
        route
            .handlers()
            .iter()
            .flat_map(|handler| Self::mapped(route.route(), handler.source()))
            .collect()
    }

    fn mapped(pattern: &str, handler: &HandlerSource) -> Vec<Self> {
        handler
            .routes()
            .iter()
            .filter(|mapping| mapping.pattern.as_deref() == Some(pattern))
            .map(|mapping| Self {
                route: String::from(pattern),
                handler: handler.clone(),
                mapping: Arc::new(mapping.clone()),
            })
            .collect()
    }
}
