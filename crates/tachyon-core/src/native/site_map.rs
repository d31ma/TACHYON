use super::config::config_failure;
use super::site::{SiteSettings, SitemapValues};
use crate::compiler::html_attribute_escape;
use crate::{Failure, RouteNode};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

pub(crate) struct SiteMap;

impl SiteMap {
    const NAMESPACE: &str = "http://www.sitemaps.org/schemas/sitemap/0.9";
    const MAX_ADDRESSES: usize = 50_000;
    const MAX_SEGMENT_LENGTH: usize = 256;

    pub(crate) fn files(
        site: &SiteSettings,
        routes: &[RouteNode],
        built: &[(PathBuf, Vec<u8>)],
    ) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        let Some(origin) = &site.origin else {
            return Ok(Vec::new());
        };
        let mut paths = Self::static_page_paths(site, routes);
        for (route, values) in &site.sitemap {
            Self::ensure_page_route(route, routes)?;
            for value in Self::listed_values(route, values, built)? {
                paths.insert(Self::fill_route(route, &value));
            }
        }
        if paths.len() > Self::MAX_ADDRESSES {
            return Err(config_failure(
                "The sitemap would list more than 50000 addresses.",
            ));
        }
        Ok(vec![
            (
                PathBuf::from("sitemap.xml"),
                Self::sitemap(origin, &paths).into_bytes(),
            ),
            (
                PathBuf::from("robots.txt"),
                Self::robots(origin).into_bytes(),
            ),
        ])
    }

    fn static_page_paths(site: &SiteSettings, routes: &[RouteNode]) -> BTreeSet<String> {
        let mut paths = BTreeSet::new();
        for route in routes {
            if route.source_path().is_none() || route.is_dynamic() {
                continue;
            }
            let hidden = site
                .metadata
                .for_route(route.route())
                .is_some_and(|metadata| metadata.is_hidden_from_search());
            if !hidden {
                paths.insert(String::from(route.route()));
            }
        }
        paths
    }

    fn ensure_page_route(route: &str, routes: &[RouteNode]) -> Result<(), Failure> {
        for node in routes {
            if node.route() == route && node.source_path().is_some() {
                return Ok(());
            }
        }
        Err(config_failure(&format!(
            "Manifest tac.sitemap names '{route}', which is not a page route."
        )))
    }

    fn listed_values(
        route: &str,
        values: &SitemapValues,
        built: &[(PathBuf, Vec<u8>)],
    ) -> Result<Vec<String>, Failure> {
        let data = &values.data;
        let document = Self::read_data(route, data, built)?;
        let Some(list) = document.pointer(&values.values).and_then(Value::as_array) else {
            return Err(config_failure(&format!(
                "'{}' in {data} is not a list, so the sitemap cannot list '{route}'.",
                values.values
            )));
        };
        let mut found = Vec::new();
        for item in list {
            found.push(Self::listed_value(route, item, values.field.as_deref())?);
        }
        Ok(found)
    }

    fn read_data(route: &str, data: &str, built: &[(PathBuf, Vec<u8>)]) -> Result<Value, Failure> {
        let wanted = data.trim_start_matches('/');
        for (path, bytes) in built {
            if path.to_string_lossy().replace('\\', "/") != wanted {
                continue;
            }
            return serde_json::from_slice(bytes).map_err(|error| {
                config_failure(&format!(
                    "{data}, which lists the values of '{route}', is not valid JSON: {error}"
                ))
            });
        }
        Err(config_failure(&format!(
            "{data}, which lists the values of '{route}', is not a file in client/shared."
        )))
    }

    fn listed_value(route: &str, item: &Value, field: Option<&str>) -> Result<String, Failure> {
        let value = match field {
            Some(field) => item.get(field),
            None => Some(item),
        };
        let Some(text) = value.and_then(Value::as_str) else {
            return Err(config_failure(&format!(
                "Each value listed for '{route}' must be a string."
            )));
        };
        if !Self::is_plain_segment(text) {
            return Err(config_failure(&format!(
                "'{text}' listed for '{route}' is not a plain path segment of letters, digits, '-', '_', '.' or '~'."
            )));
        }
        Ok(String::from(text))
    }

    fn is_plain_segment(text: &str) -> bool {
        if text.is_empty() || text.len() > Self::MAX_SEGMENT_LENGTH {
            return false;
        }
        if text == "." || text == ".." {
            return false;
        }
        text.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
    }

    fn fill_route(route: &str, value: &str) -> String {
        let mut segments = Vec::new();
        for segment in route.split('/') {
            if segment.starts_with('_') {
                segments.push(value);
            } else {
                segments.push(segment);
            }
        }
        segments.join("/")
    }

    fn sitemap(origin: &str, paths: &BTreeSet<String>) -> String {
        let mut text = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        let _ = writeln!(text, "<urlset xmlns=\"{}\">", Self::NAMESPACE);
        for path in paths {
            let address = html_attribute_escape(&format!("{origin}{path}"));
            let _ = writeln!(text, "  <url><loc>{address}</loc></url>");
        }
        text.push_str("</urlset>\n");
        text
    }

    fn robots(origin: &str) -> String {
        format!("User-agent: *\nAllow: /\n\nSitemap: {origin}/sitemap.xml\n")
    }
}
