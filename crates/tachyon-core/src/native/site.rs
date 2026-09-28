use super::config::{config_failure, manifest_head, read_manifest, valid_route};
use crate::Failure;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default)]
pub(crate) struct SiteSettings {
    pub(crate) scripts: Vec<String>,
    pub(crate) styles: Vec<String>,
    pub(crate) cache_rules: Vec<CacheRule>,
    pub(crate) metadata: SiteMetadata,
    pub(crate) manifest_head: String,
    pub(crate) origin: Option<String>,
    pub(crate) sitemap: BTreeMap<String, SitemapValues>,
    pub(crate) browser_apis: Vec<String>,
}

impl SiteSettings {
    const MAX_ORIGIN_LENGTH: usize = 256;

    pub(crate) fn load(project_root: &Path) -> Result<Self, Failure> {
        let Some(manifest) = read_manifest(project_root)? else {
            return Ok(Self::default());
        };
        let manifest_head = manifest_head(&manifest);
        let tac = manifest.into_tac();
        Self::validate_origin(tac.origin.as_deref())?;
        validate_document_paths("scripts", &tac.scripts)?;
        validate_document_paths("styles", &tac.styles)?;
        let cache_rules = compile_cache_rules(tac.cache)?;
        let browser_apis = super::browser_api_grants::BrowserApiGrants::checked(tac.browser_apis)?;
        validate_sitemap(tac.origin.as_deref(), &tac.sitemap)?;
        let mut metadata = tac.metadata;
        if let Some(origin) = &tac.origin {
            metadata.resolve_against(origin);
        }
        Ok(Self {
            scripts: tac.scripts,
            styles: tac.styles,
            cache_rules,
            metadata,
            manifest_head,
            origin: tac.origin,
            sitemap: tac.sitemap,
            browser_apis,
        })
    }

    fn validate_origin(origin: Option<&str>) -> Result<(), Failure> {
        let Some(origin) = origin else {
            return Ok(());
        };
        if Self::valid_origin(origin) {
            return Ok(());
        }
        Err(config_failure(
            "Manifest tac.origin must be the site's public address without a path, such as https://example.com.",
        ))
    }

    fn valid_origin(origin: &str) -> bool {
        if origin.len() > Self::MAX_ORIGIN_LENGTH {
            return false;
        }
        let host = match origin.strip_prefix("https://") {
            Some(host) => host,
            None => match origin.strip_prefix("http://") {
                Some(host) => host,
                None => return false,
            },
        };
        if host.is_empty() || host.chars().any(char::is_control) {
            return false;
        }
        !host.contains(['/', '?', '#', '@', '\\', '"', '<', '>', ' '])
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct PageMetadata {
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) canonical: Option<String>,
    #[serde(default)]
    pub(crate) image: Option<String>,
    #[serde(default)]
    pub(crate) site_name: Option<String>,
    #[serde(default)]
    pub(crate) lang: Option<String>,
    #[serde(default)]
    pub(crate) noindex: Option<bool>,
}

impl PageMetadata {
    fn merged_with(&self, defaults: &Self) -> Self {
        Self {
            title: self.title.clone().or_else(|| defaults.title.clone()),
            description: self
                .description
                .clone()
                .or_else(|| defaults.description.clone()),
            canonical: self
                .canonical
                .clone()
                .or_else(|| defaults.canonical.clone()),
            image: self.image.clone().or_else(|| defaults.image.clone()),
            site_name: self
                .site_name
                .clone()
                .or_else(|| defaults.site_name.clone()),
            lang: self.lang.clone().or_else(|| defaults.lang.clone()),
            noindex: self.noindex.or(defaults.noindex),
        }
    }

    fn resolve_against(&mut self, origin: &str) {
        for link in [&mut self.canonical, &mut self.image] {
            let Some(value) = link.as_mut() else {
                continue;
            };
            if value.starts_with('/') && !value.starts_with("//") {
                *value = format!("{origin}{value}");
            }
        }
    }

    pub(crate) fn is_hidden_from_search(&self) -> bool {
        self.noindex == Some(true)
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SiteMetadata {
    #[serde(default)]
    defaults: PageMetadata,
    #[serde(default)]
    routes: BTreeMap<String, PageMetadata>,
}

impl SiteMetadata {
    pub(crate) fn for_route(&self, route: &str) -> Option<PageMetadata> {
        let merged = match self.routes.get(route) {
            Some(entry) => entry.merged_with(&self.defaults),
            None => self.defaults.clone(),
        };
        if merged == PageMetadata::default() {
            return None;
        }
        Some(merged)
    }

    fn resolve_against(&mut self, origin: &str) {
        self.defaults.resolve_against(origin);
        for entry in self.routes.values_mut() {
            entry.resolve_against(origin);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SitemapValues {
    pub(crate) data: String,
    pub(crate) values: String,
    #[serde(default)]
    pub(crate) field: Option<String>,
}

fn validate_sitemap(
    origin: Option<&str>,
    sitemap: &BTreeMap<String, SitemapValues>,
) -> Result<(), Failure> {
    if !sitemap.is_empty() && origin.is_none() {
        return Err(config_failure(
            "Manifest tac.sitemap needs tac.origin, the site's public address.",
        ));
    }
    for (route, values) in sitemap {
        validate_sitemap_route(route)?;
        validate_sitemap_values(route, values)?;
    }
    Ok(())
}

fn validate_sitemap_route(route: &str) -> Result<(), Failure> {
    let mut variables = 0;
    for segment in route.split('/') {
        if segment.starts_with('_') {
            variables += 1;
        }
    }
    if valid_route(route) && variables == 1 {
        return Ok(());
    }
    Err(config_failure(&format!(
        "Manifest tac.sitemap route '{route}' must be a page route with one path variable, such as /docs/_topic."
    )))
}

fn validate_sitemap_values(route: &str, values: &SitemapValues) -> Result<(), Failure> {
    let data = &values.data;
    if !data.starts_with("/shared/") || data.contains(['?', '#']) || !local_document_path(data) {
        return Err(config_failure(&format!(
            "Manifest tac.sitemap data '{data}' for '{route}' must be a JSON file under /shared/, such as /shared/data/topics.json."
        )));
    }
    if !values.values.is_empty() && !values.values.starts_with('/') {
        return Err(config_failure(&format!(
            "Manifest tac.sitemap values for '{route}' must be a JSON pointer to a list, such as /topics."
        )));
    }
    if values.field.as_deref().is_some_and(str::is_empty) {
        return Err(config_failure(&format!(
            "Manifest tac.sitemap field for '{route}' must name a member of each listed object."
        )));
    }
    Ok(())
}

struct DocumentPath;

impl DocumentPath {
    const MAX_ENTRIES: usize = 8;
    const MAX_LENGTH: usize = 256;
}

fn validate_document_paths(member: &str, declared: &[String]) -> Result<(), Failure> {
    if declared.len() > DocumentPath::MAX_ENTRIES {
        return Err(config_failure(&format!(
            "Manifest tac.{member} lists more than 8 entries."
        )));
    }
    for source in declared {
        if !local_document_path(source) {
            return Err(config_failure(&format!(
                "'{source}' in tac.{member} must be a local absolute project path of at most 256 characters, without ambiguous separators, traversal, or control characters."
            )));
        }
    }
    Ok(())
}

fn local_document_path(source: &str) -> bool {
    if source.len() > DocumentPath::MAX_LENGTH || source.chars().any(char::is_control) {
        return false;
    }
    let path = source.split(['?', '#']).next().unwrap_or(source);
    if !path.starts_with('/') || path.starts_with("//") || path.contains('\\') {
        return false;
    }
    let normalized = path.to_ascii_lowercase().replace("%2e", ".");
    !normalized.contains("..") && !normalized.contains("%2f") && !normalized.contains("%5c")
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct CacheRule {
    pub(crate) pattern: String,
    pub(crate) policy: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawCacheRule {
    path: String,
    policy: String,
}

impl RawCacheRule {
    const MAX_RULES: usize = 64;
    const MAX_PATH_LENGTH: usize = 256;
    const POLICIES: [&str; 3] = ["cache-first", "network-first", "no-store"];
    const WILDCARD_PATTERN: &str = "[^?]*";

    fn path_character_is_allowed(character: char) -> bool {
        character.is_ascii_alphanumeric() || matches!(character, '/' | '-' | '_' | '.')
    }
}

fn compile_cache_rules(declared: Vec<RawCacheRule>) -> Result<Vec<CacheRule>, Failure> {
    if declared.len() > RawCacheRule::MAX_RULES {
        return Err(config_failure(
            "Manifest tac.cache declares more than 64 cache rules.",
        ));
    }
    let mut rules = Vec::with_capacity(declared.len());
    for raw in declared {
        rules.push(cache_rule(raw)?);
    }
    Ok(rules)
}

fn cache_rule(raw: RawCacheRule) -> Result<CacheRule, Failure> {
    if !RawCacheRule::POLICIES.contains(&raw.policy.as_str()) {
        return Err(config_failure(&format!(
            "'{}' is not a cache policy. Use cache-first, network-first or no-store.",
            raw.policy
        )));
    }
    if !raw.path.starts_with('/') || raw.path.len() > RawCacheRule::MAX_PATH_LENGTH {
        return Err(config_failure(&format!(
            "Cache path '{}' must be an absolute path of at most 256 characters.",
            raw.path
        )));
    }
    let mut pattern = String::from("^");
    for (index, part) in raw.path.split('*').enumerate() {
        if index > 0 {
            pattern.push_str(RawCacheRule::WILDCARD_PATTERN);
        }
        for character in part.chars() {
            if !RawCacheRule::path_character_is_allowed(character) {
                return Err(config_failure(&format!(
                    "Cache path '{}' may contain only letters, digits, '/', '-', '_', '.' and '*'.",
                    raw.path
                )));
            }
            if character == '.' {
                pattern.push('\\');
            }
            pattern.push(character);
        }
    }
    pattern.push('$');
    Ok(CacheRule {
        pattern,
        policy: raw.policy,
    })
}
