use super::{HandlerNode, RouteNode, ViewSource};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeMap;
use tachyon_contracts::RouteManifest;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteGraph {
    routes: Vec<RouteNode>,
}

impl RouteGraph {
    #[must_use]
    pub fn routes(&self) -> &[RouteNode] {
        &self.routes
    }

    #[must_use]
    pub fn manifest(&self) -> RouteManifest {
        let routes = self.routes.iter().map(RouteNode::manifest_entry).collect();
        RouteManifest::v1(routes)
    }

    pub(super) fn build(
        views: Vec<(String, ViewSource)>,
        handlers: Vec<(String, HandlerNode)>,
    ) -> Result<Self, Failure> {
        let mut routes: BTreeMap<String, RouteNode> = BTreeMap::new();
        views
            .into_iter()
            .try_for_each(|(route, view)| Self::add_view(&mut routes, &route, view))?;
        Self::add_handlers(&mut routes, handlers);
        routes.values_mut().try_for_each(|route| {
            route.sort_sources();
            route.check_mapped_verbs()
        })?;
        Ok(Self {
            routes: routes.into_values().collect(),
        })
    }

    fn add_view(
        routes: &mut BTreeMap<String, RouteNode>,
        route: &str,
        view: ViewSource,
    ) -> Result<(), Failure> {
        let node = Self::node(routes, route);
        if let Some(existing) = &node.view {
            return Err(Self::claimed_twice(route, existing, &view));
        }
        node.view = Some(view);
        Ok(())
    }

    fn add_handlers(
        routes: &mut BTreeMap<String, RouteNode>,
        handlers: Vec<(String, HandlerNode)>,
    ) {
        for (route, handler) in handlers {
            Self::node(routes, &route).handlers.push(handler);
        }
    }

    fn node<'a>(routes: &'a mut BTreeMap<String, RouteNode>, route: &str) -> &'a mut RouteNode {
        routes
            .entry(String::from(route))
            .or_insert_with(|| RouteNode::empty(String::from(route)))
    }

    fn claimed_twice(route: &str, existing: &ViewSource, view: &ViewSource) -> Failure {
        Failure::one(diagnostic(
            1003,
            format!(
                "Route '{}' is claimed by both '{}' and '{}'.",
                route, existing.source_path, view.source_path
            ),
            Some(String::from("Keep exactly one Tac view for each route.")),
            source_span(&view.source_path, 0, view.source_path.len()),
        ))
    }
}
