mod route_method;
mod route_path;

pub(crate) use route_method::RouteMethod;
pub(crate) use route_path::RoutePath;

use std::cmp::Reverse;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteMatch {
    pub route: String,
    pub parameters: BTreeMap<String, String>,
}

impl RouteMatch {
    const MAX_SEGMENTS: usize = 64;
    const MAX_PARAMETER_BYTES: usize = 1_024;

    pub(crate) fn most_specific<'a>(
        routes: impl IntoIterator<Item = &'a str>,
        path: &str,
    ) -> Option<Self> {
        let actual = Self::segments(path)?;
        routes
            .into_iter()
            .filter_map(|route| Self::candidate(route, &actual))
            .min_by_key(|(specificity, _)| Reverse(*specificity))
            .map(|(_, matched)| matched)
    }

    fn candidate(route: &str, actual: &[&str]) -> Option<(u32, Self)> {
        let (parameters, specificity) = Self::bind(route, actual)?;
        let matched = Self {
            route: String::from(route),
            parameters,
        };
        Some((specificity, matched))
    }

    fn segments(path: &str) -> Option<Vec<&str>> {
        let parts = Self::split_parts(path.trim_matches('/'));
        Self::usable_parts(parts)
    }

    fn split_parts(trimmed: &str) -> Vec<&str> {
        if trimmed.is_empty() {
            return Vec::new();
        }
        trimmed.split('/').collect()
    }

    fn usable_parts(parts: Vec<&str>) -> Option<Vec<&str>> {
        let usable = parts.len() <= Self::MAX_SEGMENTS && !parts.contains(&"");
        usable.then_some(parts)
    }

    fn bind(pattern: &str, path: &[&str]) -> Option<(BTreeMap<String, String>, u32)> {
        let pattern_segments = Self::segments(pattern)?;
        (pattern_segments.len() == path.len()).then_some(())?;
        let mut parameters = BTreeMap::new();
        let mut specificity = 0;
        pattern_segments
            .iter()
            .zip(path)
            .try_for_each(|(pattern_segment, actual)| {
                Self::bind_segment(pattern_segment, actual, &mut parameters, &mut specificity)
            })?;
        Some((parameters, specificity))
    }

    fn bind_segment(
        pattern_segment: &str,
        actual: &str,
        parameters: &mut BTreeMap<String, String>,
        specificity: &mut u32,
    ) -> Option<()> {
        pattern_segment.strip_prefix('_').map_or_else(
            || Self::match_static(pattern_segment, actual, specificity),
            |name| Self::bind_parameter(name, actual, parameters),
        )
    }

    fn match_static(pattern_segment: &str, actual: &str, specificity: &mut u32) -> Option<()> {
        if pattern_segment != actual {
            return None;
        }
        *specificity += 1;
        Some(())
    }

    fn bind_parameter(
        name: &str,
        actual: &str,
        parameters: &mut BTreeMap<String, String>,
    ) -> Option<()> {
        (1..=Self::MAX_PARAMETER_BYTES)
            .contains(&actual.len())
            .then_some(())?;
        let decoded = Self::percent_decode(actual)?;
        parameters.insert(String::from(name), decoded);
        Some(())
    }

    fn percent_decode(value: &str) -> Option<String> {
        let decoded = Self::decode_bytes(value.as_bytes())?;
        let text = String::from_utf8(decoded).ok()?;
        Self::safe_text(text)
    }

    fn decode_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
        let mut decoded = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            index += Self::decode_at(bytes, index, &mut decoded)?;
        }
        Some(decoded)
    }

    fn decode_at(bytes: &[u8], index: usize, decoded: &mut Vec<u8>) -> Option<usize> {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            return Some(1);
        }
        decoded.push(Self::escaped_byte(bytes, index)?);
        Some(3)
    }

    fn escaped_byte(bytes: &[u8], index: usize) -> Option<u8> {
        let high = bytes.get(index + 1).and_then(|byte| Self::hex(*byte))?;
        let low = bytes.get(index + 2).and_then(|byte| Self::hex(*byte))?;
        Some((high << 4) | low)
    }

    const fn hex(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    fn safe_text(text: String) -> Option<String> {
        let unsafe_text =
            text.contains('/') || text.contains('\\') || text.chars().any(char::is_control);
        (!unsafe_text).then_some(text)
    }
}

#[must_use]
pub fn match_route<'a, I>(routes: I, path: &str) -> Option<RouteMatch>
where
    I: IntoIterator<Item = &'a str>,
{
    RouteMatch::most_specific(routes, path)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{RouteMatch, RoutePath, match_route};

    #[test]
    fn a_route_path_becomes_an_internal_pattern() {
        assert_eq!(RoutePath::pattern("", "").as_deref(), Ok("/"));
        assert_eq!(RoutePath::pattern("/", "").as_deref(), Ok("/"));
        assert_eq!(RoutePath::pattern("/orders", "").as_deref(), Ok("/orders"));
        assert_eq!(
            RoutePath::pattern("/orders", "/{id}/lines").as_deref(),
            Ok("/orders/_id/lines")
        );
        assert_eq!(RoutePath::template("/orders/_id"), "/orders/{id}");
    }

    #[test]
    fn a_bad_route_path_is_explained() {
        let cases = [
            ("orders", "must start with '/'"),
            ("/orders/", "empty part"),
            ("/Orders", "lowercase letters"),
            ("/orders/{Id}", "path variable '{Id}'"),
            ("/a/{id}/b/{id}", "appears twice"),
            ("/_secret", "lowercase letters"),
        ];
        for (path, expected) in cases {
            let error = RoutePath::pattern("", path).expect_err(path);
            assert!(error.contains(expected), "{path}: {error}");
        }
    }

    fn matched(path: &str) -> Option<RouteMatch> {
        let routes = [
            "/",
            "/items",
            "/items/_id",
            "/items/new",
            "/items/_id/reviews/_review",
        ];
        match_route(routes, path)
    }

    #[test]
    fn static_routes_win_over_dynamic_ones_in_the_same_position() {
        let new = matched("/items/new").expect("static match");
        assert_eq!(new.route, "/items/new");
        assert!(new.parameters.is_empty());

        let dynamic = matched("/items/42").expect("dynamic match");
        assert_eq!(dynamic.route, "/items/_id");
        assert_eq!(dynamic.parameters["id"], "42");
    }

    #[test]
    fn nested_parameters_bind_in_order() {
        let nested = matched("/items/42/reviews/7").expect("nested match");
        assert_eq!(nested.route, "/items/_id/reviews/_review");
        assert_eq!(nested.parameters["id"], "42");
        assert_eq!(nested.parameters["review"], "7");
    }

    #[test]
    fn index_and_trailing_slashes_resolve_to_one_route() {
        for path in ["/", "", "//"] {
            assert_eq!(matched(path).expect("index").route, "/");
        }
        assert_eq!(matched("/items/").expect("trailing").route, "/items");
    }

    #[test]
    fn parameters_are_percent_decoded() {
        let decoded = matched("/items/a%20b").expect("decoded");
        assert_eq!(decoded.parameters["id"], "a b");
    }

    #[test]
    fn traversal_and_control_characters_never_bind() {
        for path in [
            "/items/%2e%2e%2fetc",
            "/items/%2f",
            "/items/%5c",
            "/items/%00",
        ] {
            assert!(matched(path).is_none(), "{path} bound a parameter");
        }
        assert!(matched("/items/../secret").is_none());
    }

    #[test]
    fn arity_and_unknown_paths_do_not_match() {
        assert!(matched("/items/42/extra").is_none());
        assert!(matched("/unknown").is_none());
        assert!(matched("/items/42/reviews").is_none());
    }

    #[test]
    fn oversized_paths_are_rejected() {
        let deep = format!("/{}", vec!["a"; 128].join("/"));
        assert!(matched(&deep).is_none());
        let long = format!("/items/{}", "x".repeat(2_048));
        assert!(matched(&long).is_none());
    }
}
