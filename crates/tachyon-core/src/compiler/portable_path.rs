use std::path::{Component, Path};

pub(crate) struct PortablePath;

impl PortablePath {
    pub(crate) fn of(path: &Path) -> String {
        let segments: Vec<&str> = path.components().filter_map(Self::normal_text).collect();
        segments.join("/")
    }

    pub(super) fn route_key(route: &str) -> String {
        Self::key_for_route(route)
    }

    fn normal_text(component: Component<'_>) -> Option<&str> {
        match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        }
    }

    fn key_for_route(route: &str) -> String {
        if route == "/" {
            return String::from("root");
        }
        route.trim_start_matches('/').replace('/', "__")
    }
}
