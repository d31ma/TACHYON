#![allow(clippy::expect_used)]

use super::site::{PageMetadata, SiteSettings};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

fn write_tac(root: &Path, tac: &Value) {
    fs::create_dir_all(root.join("client")).expect("client");
    let manifest = json!({ "name": "Shop", "tac": tac });
    fs::write(root.join("client/manifest.json"), manifest.to_string()).expect("manifest");
}

fn load(tac: &Value) -> Result<SiteSettings, String> {
    let root = tempfile::tempdir().expect("project");
    write_tac(root.path(), tac);
    SiteSettings::load(root.path()).map_err(|failure| failure.to_string())
}

fn load_error(tac: &Value) -> String {
    let Err(text) = load(tac) else {
        panic!("{tac} was accepted");
    };
    assert!(text.contains("TY1601"), "{text}");
    text
}

fn cache_patterns(path: &str, policy: &str) -> Result<Vec<String>, String> {
    let settings = load(&json!({ "cache": [{ "path": path, "policy": policy }] }))?;
    let mut patterns = Vec::new();
    for rule in settings.cache_rules {
        patterns.push(rule.pattern);
    }
    Ok(patterns)
}

#[test]
fn a_project_without_a_manifest_has_empty_site_settings() {
    let root = tempfile::tempdir().expect("project");
    let settings = SiteSettings::load(root.path()).expect("settings");
    assert!(settings.scripts.is_empty());
    assert!(settings.manifest_head.is_empty());
    assert!(settings.origin.is_none());
    assert!(settings.metadata.for_route("/").is_none());
}

#[test]
fn scripts_styles_and_origin_are_read_from_the_manifest() {
    let settings = load(&json!({
        "origin": "https://shop.example",
        "scripts": ["/shared/app.js?v=1&next=https://example.invalid/a..b&encoded=%2f#section"],
        "blockingScripts": ["/shared/scripts/ThemeDefault.js"],
        "styles": ["/shared/name%20with%20spaces.css"],
        "api": { "url": "/api" },
    }))
    .expect("settings");
    assert_eq!(
        settings.scripts,
        vec![String::from(
            "/shared/app.js?v=1&next=https://example.invalid/a..b&encoded=%2f#section"
        )]
    );
    assert_eq!(
        settings.blocking_scripts,
        vec![String::from("/shared/scripts/ThemeDefault.js")]
    );
    assert_eq!(
        settings.styles,
        vec![String::from("/shared/name%20with%20spaces.css")]
    );
    assert_eq!(settings.origin.as_deref(), Some("https://shop.example"));
    assert!(settings.manifest_head.contains("manifest.webmanifest"));
}

#[test]
fn document_resources_reject_foreign_and_ambiguous_paths() {
    for source in [
        "//example.invalid/module.js",
        "/\\example.invalid/module.js",
        "/\t/example.invalid/module.js",
        "/shared/../module.js",
        "/shared/%2e%2E/module.js",
        "/shared/%2fmodule.js",
        "/shared/%5Cmodule.js",
    ] {
        for member in ["scripts", "blockingScripts", "styles"] {
            let text = load_error(&json!({ member: [source] }));
            assert!(text.contains(&format!("in tac.{member}")), "{text}");
        }
    }
    let text = load_error(&json!({ "scripts": vec!["/shared/a.js"; 9] }));
    assert!(text.contains("more than 8 entries"), "{text}");
    let text = load_error(&json!({ "blockingScripts": vec!["/shared/a.js"; 9] }));
    assert!(
        text.contains("tac.blockingScripts lists more than 8 entries"),
        "{text}"
    );
}

#[test]
fn an_origin_must_be_a_bare_site_address() {
    for origin in [
        "shop.example",
        "ftp://shop.example",
        "https://",
        "https://shop.example/",
        "https://shop.example/path",
        "https://user@shop.example",
        "https://shop.example?x=1",
    ] {
        let text = load_error(&json!({ "origin": origin }));
        assert!(text.contains("tac.origin"), "{origin}: {text}");
    }
    assert!(load(&json!({ "origin": "http://127.0.0.1:4700" })).is_ok());
}

#[test]
fn a_declared_cache_path_compiles_to_an_anchored_expression() {
    assert_eq!(
        cache_patterns("/api/products", "cache-first"),
        Ok(vec![String::from("^/api/products$")])
    );
    assert_eq!(
        cache_patterns("/api/search/*", "network-first"),
        Ok(vec![String::from("^/api/search/[^?]*$")])
    );
    assert_eq!(
        cache_patterns("/api/v1.0/items", "no-store"),
        Ok(vec![String::from(r"^/api/v1\.0/items$")])
    );
}

#[test]
fn only_declared_cache_policies_and_safe_paths_are_accepted() {
    assert!(cache_patterns("/api/products", "stale-while-revalidate").is_err());
    assert!(cache_patterns("api/products", "cache-first").is_err());
    for path in ["/api/(a|b)", "/api/a+", "/api/^x", "/api/a$"] {
        assert!(cache_patterns(path, "cache-first").is_err(), "{path}");
    }
    let text =
        load_error(&json!({ "cache": [{ "path": "/a", "policy": "no-store", "extra": 1 }] }));
    assert!(text.contains("unknown field `extra`"), "{text}");
}

#[test]
fn page_metadata_merges_site_defaults_and_resolves_local_links() {
    let settings = load(&json!({
        "origin": "https://shop.example",
        "metadata": {
            "defaults": { "image": "/shared/logo.svg", "siteName": "Shop", "lang": "en" },
            "routes": {
                "/": { "title": "Home", "canonical": "/" },
                "/docs/_topic": { "title": "Guide", "canonical": "https://docs.example/x", "lang": "fr", "noindex": true },
            },
        },
    }))
    .expect("settings");
    let home = settings.metadata.for_route("/").expect("home");
    assert_eq!(home.title.as_deref(), Some("Home"));
    assert_eq!(home.canonical.as_deref(), Some("https://shop.example/"));
    assert_eq!(
        home.image.as_deref(),
        Some("https://shop.example/shared/logo.svg")
    );
    assert_eq!(home.site_name.as_deref(), Some("Shop"));
    assert!(!home.is_hidden_from_search());
    let topic = settings.metadata.for_route("/docs/_topic").expect("topic");
    assert_eq!(topic.canonical.as_deref(), Some("https://docs.example/x"));
    assert_eq!(topic.lang.as_deref(), Some("fr"));
    assert!(topic.is_hidden_from_search());
    let other = settings.metadata.for_route("/other").expect("defaults");
    assert_eq!(other.title, None);
    assert_eq!(other.site_name.as_deref(), Some("Shop"));
}

#[test]
fn a_site_without_metadata_adds_nothing_to_a_page() {
    let settings = load(&json!({ "origin": "https://shop.example" })).expect("settings");
    assert_eq!(settings.metadata.for_route("/"), None::<PageMetadata>);
}

#[test]
fn metadata_fields_are_checked_by_name() {
    let text = load_error(&json!({ "metadata": { "routes": { "/": { "site_name": "Shop" } } } }));
    assert!(text.contains("unknown field `site_name`"), "{text}");
    let text = load_error(&json!({ "metadata": { "/": { "title": "Home" } } }));
    assert!(text.contains("unknown field `/`"), "{text}");
}

#[test]
fn sitemap_entries_are_checked_before_the_build() {
    let values = json!({ "data": "/shared/data/items.json", "values": "/items" });
    let text = load_error(&json!({ "sitemap": { "/items/_id": values } }));
    assert!(text.contains("needs tac.origin"), "{text}");
    for (route, entry, expected) in [
        ("/items", values.clone(), "one path variable"),
        ("/items/_a/_b", values.clone(), "one path variable"),
        (
            "/items/_id",
            json!({ "data": "/data/items.json", "values": "/items" }),
            "a JSON file under /shared/",
        ),
        (
            "/items/_id",
            json!({ "data": "/shared/../items.json", "values": "/items" }),
            "a JSON file under /shared/",
        ),
        (
            "/items/_id",
            json!({ "data": "/shared/items.json", "values": "items" }),
            "a JSON pointer",
        ),
        (
            "/items/_id",
            json!({ "data": "/shared/items.json", "values": "/items", "field": "" }),
            "must name a member",
        ),
        (
            "/items/_id",
            json!({ "data": "/shared/items.json", "values": "/items", "key": "id" }),
            "unknown field `key`",
        ),
    ] {
        let text = load_error(&json!({
            "origin": "https://shop.example",
            "sitemap": { route: entry },
        }));
        assert!(text.contains(expected), "{route}: {text}");
    }
}

#[test]
fn the_manifest_grants_browser_apis_by_name() {
    let settings =
        load(&json!({ "browserApis": ["clipboard", "geolocation", "preferenceStrings"] }))
            .expect("settings");
    assert_eq!(
        settings.browser_apis,
        ["clipboard", "geolocation", "preferenceStrings"]
    );
    let unknown = load_error(&json!({ "browserApis": ["camera"] }));
    assert!(
        unknown.contains("'camera' in tac.browserApis is not a browser API that needs a grant. Grant any of: clipboard, geolocation, notifications, preferenceStrings, share."),
        "{unknown}"
    );
    let twice = load_error(&json!({ "browserApis": ["share", "share"] }));
    assert!(
        twice.contains("lists a browser API more than once"),
        "{twice}"
    );
}

#[test]
fn the_manifest_theme_color_reaches_every_page_head() {
    let manifest = json!({ "name": "Shop", "theme_color": "#0b5fff", "tac": {} });
    let bytes = manifest.to_string().into_bytes();
    let settings = SiteSettings::from_captured_manifest(Some(&bytes)).expect("settings");
    assert!(
        settings
            .manifest_head
            .ends_with(r##"<meta name="theme-color" content="#0b5fff">"##),
        "{}",
        settings.manifest_head
    );
}

#[test]
fn a_declared_yon_origin_is_read_from_the_manifest() {
    let settings =
        load(&json!({ "yon": { "origin": "https://api.example.com" } })).expect("settings");
    assert_eq!(
        settings.yon_origin.as_deref(),
        Some("https://api.example.com")
    );
}

#[test]
fn a_yon_origin_that_is_not_an_origin_stops_the_build() {
    for yon in [
        json!({ "origin": "https://api.example.com/" }),
        json!({ "origin": "http://api.example.com" }),
        json!({ "url": "https://api.example.com" }),
    ] {
        let text = load_error(&json!({ "yon": yon }));
        assert!(
            text.contains("tac.yon") || text.contains("unknown field"),
            "{text}"
        );
    }
}
