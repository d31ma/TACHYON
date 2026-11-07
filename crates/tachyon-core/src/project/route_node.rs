use super::{HandlerNode, ViewKind, ViewSource};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::routing::{RouteMethod, RoutePath};
use crate::yon::plan::Route;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tachyon_contracts::{HttpMethod, RouteContext, RouteEntry, RouteKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteNode {
    route: String,
    pub(super) view: Option<ViewSource>,
    pub(super) handlers: Vec<HandlerNode>,
}

impl RouteNode {
    pub(super) const fn empty(route: String) -> Self {
        Self {
            route,
            view: None,
            handlers: Vec::new(),
        }
    }

    #[must_use]
    pub fn route(&self) -> &str {
        &self.route
    }

    #[must_use]
    pub fn source_path(&self) -> Option<&str> {
        self.view.as_ref().map(|view| view.source_path.as_str())
    }

    #[must_use]
    pub fn absolute_source_path(&self) -> Option<&Path> {
        self.view
            .as_ref()
            .map(|view| view.absolute_source_path.as_path())
    }

    #[must_use]
    pub fn view_kind(&self) -> Option<ViewKind> {
        self.view.as_ref().map(|view| view.view_kind)
    }

    pub(crate) fn view_bytes(&self) -> Option<&[u8]> {
        self.view.as_ref().map(|view| view.bytes.as_slice())
    }

    #[must_use]
    pub fn handlers(&self) -> &[HandlerNode] {
        &self.handlers
    }

    #[must_use]
    pub fn parameters(&self) -> Vec<String> {
        self.route
            .split('/')
            .filter_map(|segment| segment.strip_prefix('_'))
            .map(String::from)
            .collect()
    }

    #[must_use]
    pub fn is_dynamic(&self) -> bool {
        self.route
            .split('/')
            .any(|segment| segment.starts_with('_'))
    }

    #[must_use]
    pub fn output_path(&self) -> Option<PathBuf> {
        self.template_output_path().filter(|_| !self.is_dynamic())
    }

    #[must_use]
    pub fn template_output_path(&self) -> Option<PathBuf> {
        self.view.as_ref()?;
        let mut path: PathBuf = self
            .route
            .trim_start_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        path.push("index.html");
        Some(path)
    }

    pub(crate) fn mapped_verbs(&self) -> Vec<HttpMethod> {
        let verbs = self
            .handler_routes()
            .filter_map(|(_, route)| RouteMethod::parse(&route.verb))
            .flat_map(Self::with_head)
            .collect();
        Self::with_options(verbs)
    }

    pub(super) fn sort_sources(&mut self) {
        self.handlers
            .sort_by(|left, right| left.source_path.cmp(&right.source_path));
    }

    pub(super) fn manifest_entry(&self) -> RouteEntry {
        RouteEntry {
            route: self.route.clone(),
            parameters: self.parameters(),
            kind: self.manifest_kind(),
            methods: self.manifest_methods(),
            view: self.source_path().map(String::from),
            handlers: self
                .handlers
                .iter()
                .map(HandlerNode::manifest_handler)
                .collect(),
            context: RouteContext::default(),
        }
    }

    pub(super) fn check_mapped_verbs(&self) -> Result<(), Failure> {
        let mut owners: BTreeMap<&str, &str> = BTreeMap::new();
        self.handler_routes()
            .try_for_each(|(handler, route)| self.claim_verb(&mut owners, handler, route))
    }

    fn handler_routes(&self) -> impl Iterator<Item = (&HandlerNode, &Route)> + '_ {
        self.handlers
            .iter()
            .flat_map(|handler| {
                handler
                    .source()
                    .routes()
                    .iter()
                    .map(move |route| (handler, route))
            })
            .filter(|(_, route)| route.pattern.as_deref() == Some(self.route.as_str()))
    }

    fn with_head(verb: HttpMethod) -> Vec<HttpMethod> {
        if verb == HttpMethod::Get {
            return vec![verb, HttpMethod::Head];
        }
        vec![verb]
    }

    fn with_options(mut verbs: Vec<HttpMethod>) -> Vec<HttpMethod> {
        if verbs.is_empty() {
            return verbs;
        }
        verbs.push(HttpMethod::Options);
        verbs.sort_by_key(|verb| RouteMethod::name(*verb));
        verbs.dedup();
        verbs
    }

    fn claim_verb<'a>(
        &self,
        owners: &mut BTreeMap<&'a str, &'a str>,
        handler: &'a HandlerNode,
        route: &'a Route,
    ) -> Result<(), Failure> {
        match owners.insert(&route.verb, &handler.source_path) {
            Some(first) if first != handler.source_path => {
                Err(self.answered_twice(&route.verb, first, handler))
            }
            _ => Ok(()),
        }
    }

    fn answered_twice(&self, verb: &str, first: &str, handler: &HandlerNode) -> Failure {
        let path = RoutePath::template(&self.route);
        Failure::one(diagnostic(
            1003,
            format!(
                "Two controllers answer {verb} {path}: '{first}' and '{}'.",
                handler.source_path
            ),
            Some(String::from(
                "Keep exactly one controller method for each route path and HTTP method.",
            )),
            source_span(&handler.source_path, 0, handler.source_path.len()),
        ))
    }

    const fn manifest_kind(&self) -> RouteKind {
        if self.view.is_some() {
            RouteKind::Page
        } else {
            RouteKind::Api
        }
    }

    fn manifest_methods(&self) -> Vec<HttpMethod> {
        let mapped = self.mapped_verbs();
        if !mapped.is_empty() {
            return mapped;
        }
        vec![HttpMethod::Get, HttpMethod::Head]
    }
}
