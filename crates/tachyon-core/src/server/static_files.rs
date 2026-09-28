use super::HotUpdateHub;
use crate::routing::{RouteMatch, match_route};
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, Response, StatusCode, Uri};
use serde_json::Value;
use std::path::Path;
use tower::ServiceExt as _;
use tower_http::services::ServeDir;

pub(in crate::server) struct StaticFiles;

impl StaticFiles {
    const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;

    pub(in crate::server) fn directory(root: &Path, precompressed: bool) -> ServeDir {
        let files = ServeDir::new(root).append_index_html_on_directories(true);
        Self::with_variants(files, precompressed)
    }

    fn with_variants(files: ServeDir, precompressed: bool) -> ServeDir {
        if precompressed {
            return files.precompressed_br().precompressed_gzip();
        }
        files
    }

    pub(in crate::server) async fn serve(
        files: &ServeDir,
        patterns: &[String],
        request: Request<Body>,
    ) -> Option<Response<Body>> {
        let mut fallback = Self::request_without_body(&request);
        let response = files.clone().oneshot(request).await.ok()?.map(Body::new);
        let missing = response.status() == StatusCode::NOT_FOUND;
        let rewritten = Self::rewrite_when_missing(missing, patterns, &mut fallback);
        Self::serve_fallback(files, response, rewritten, fallback).await
    }

    pub(in crate::server) fn is_html_document(response: &Response<Body>) -> bool {
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/html"))
    }

    pub(in crate::server) fn inject_live_reload(document: &str) -> String {
        let document =
            document.replacen("<body", "<body data-tachyon-environment=\"development\"", 1);
        let script_tag = HotUpdateHub::LIVE_RELOAD_SCRIPT_TAG;
        document.rfind("</body>").map_or_else(
            || format!("{document}{script_tag}"),
            |position| {
                format!(
                    "{}{script_tag}{}",
                    &document[..position],
                    &document[position..]
                )
            },
        )
    }

    pub(in crate::server) fn preview_route_patterns(root: &Path) -> Vec<String> {
        let path = root.join("route-manifest.json");
        Self::read_manifest(&path)
            .map(|manifest| Self::manifest_routes(&manifest))
            .unwrap_or_default()
    }

    fn rewrite_when_missing(
        missing: bool,
        patterns: &[String],
        request: &mut Request<Body>,
    ) -> bool {
        if missing {
            return Self::rewrite_dynamic_request(patterns, request);
        }
        false
    }

    async fn serve_fallback(
        files: &ServeDir,
        response: Response<Body>,
        rewritten: bool,
        fallback: Request<Body>,
    ) -> Option<Response<Body>> {
        if !rewritten {
            return Some(response);
        }
        let response = files.clone().oneshot(fallback).await.ok()?;
        Some(response.map(Body::new))
    }

    fn request_without_body(request: &Request<Body>) -> Request<Body> {
        let mut copy = Request::new(Body::empty());
        *copy.method_mut() = request.method().clone();
        *copy.uri_mut() = request.uri().clone();
        *copy.version_mut() = request.version();
        *copy.headers_mut() = request.headers().clone();
        copy
    }

    fn rewrite_dynamic_request(patterns: &[String], request: &mut Request<Body>) -> bool {
        let path = request.uri().path();
        let rewritten = match_route(patterns.iter().map(String::as_str), path)
            .and_then(|matched| Self::dynamic_uri(&matched, path, request.uri().query()));
        Self::replace_uri(request, rewritten)
    }

    fn dynamic_uri(matched: &RouteMatch, path: &str, query: Option<&str>) -> Option<Uri> {
        let dynamic = Self::is_dynamic(matched, path);
        dynamic
            .then(|| Self::rewritten_uri(matched, query))
            .flatten()
    }

    fn is_dynamic(matched: &RouteMatch, path: &str) -> bool {
        matched.route != path && matched.route.contains("/_")
    }

    fn rewritten_uri(matched: &RouteMatch, query: Option<&str>) -> Option<Uri> {
        let query = query.map_or_else(String::new, |value| format!("?{value}"));
        let rewritten = format!("{}/{}", matched.route.trim_end_matches('/'), query);
        rewritten.parse().ok()
    }

    fn replace_uri(request: &mut Request<Body>, uri: Option<Uri>) -> bool {
        if let Some(uri) = uri {
            *request.uri_mut() = uri;
            return true;
        }
        false
    }

    fn read_manifest(path: &Path) -> Option<Value> {
        let metadata = std::fs::symlink_metadata(path).ok()?;
        Some(metadata).filter(Self::small_regular_file)?;
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice::<Value>(&bytes).ok()
    }

    fn small_regular_file(metadata: &std::fs::Metadata) -> bool {
        !metadata.file_type().is_symlink()
            && metadata.is_file()
            && metadata.len() <= Self::MAX_MANIFEST_BYTES
    }

    fn manifest_routes(manifest: &Value) -> Vec<String> {
        manifest
            .get("routes")
            .and_then(Value::as_array)
            .map(|routes| {
                routes
                    .iter()
                    .filter_map(|entry| entry.get("route").and_then(Value::as_str))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default()
    }
}
