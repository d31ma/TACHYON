use super::StaticFiles;
use crate::{BuildOptions, WebCompiler};
use axum::body::{Body, to_bytes};
use axum::http::header::{
    ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH, RANGE,
    VARY,
};
use axum::http::{Method, Request, Response, StatusCode};
use std::fs;
use std::io::Read;
use std::path::Path;
use tempfile::TempDir;
use tower_http::services::ServeDir;

struct PublishedSite {
    _project: TempDir,
    files: ServeDir,
    identity: ServeDir,
    output: std::path::PathBuf,
}

impl PublishedSite {
    const RUNTIME: &'static str = "/.tachyon/tac-client.js";
    const BODY_LIMIT: usize = 16 * 1_048_576;

    async fn build() -> Self {
        let project = tempfile::tempdir().expect("project");
        let paragraphs = "<p>Tachyon serves compressed files.</p>".repeat(64);
        for (relative, contents) in [
            ("client/pages/tac.html", String::from("<page-view />")),
            (
                "client/components/page/view/tac.html",
                format!("<main>{paragraphs}</main>"),
            ),
        ] {
            let source = project.path().join(relative);
            fs::create_dir_all(source.parent().expect("parent")).expect("folder");
            fs::write(source, contents).expect("source");
        }
        let built = WebCompiler::build_async(project.path(), &BuildOptions::default())
            .await
            .expect("build");
        let output = built.output_directory().to_path_buf();
        Self {
            files: StaticFiles::directory(&output, true),
            identity: StaticFiles::directory(&output, false),
            output,
            _project: project,
        }
    }

    async fn send(
        files: &ServeDir,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
    ) -> Response<Body> {
        let mut request = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let request = request.body(Body::empty()).expect("request");
        StaticFiles::serve(files, &[], request)
            .await
            .expect("response")
    }

    async fn get(&self, uri: &str, headers: &[(&str, &str)]) -> Response<Body> {
        Self::send(&self.files, Method::GET, uri, headers).await
    }

    fn original(&self, uri: &str) -> Vec<u8> {
        fs::read(self.output.join(uri.trim_start_matches('/'))).expect("original file")
    }

    fn header<'a>(response: &'a Response<Body>, name: &axum::http::HeaderName) -> &'a str {
        response
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
    }

    async fn body(response: Response<Body>) -> Vec<u8> {
        to_bytes(response.into_body(), Self::BODY_LIMIT)
            .await
            .expect("body")
            .to_vec()
    }

    fn decoded(encoding: &str, bytes: &[u8]) -> Vec<u8> {
        let mut decoded = Vec::new();
        match encoding {
            "br" => brotli::Decompressor::new(bytes, 4096)
                .read_to_end(&mut decoded)
                .expect("brotli"),
            "gzip" => flate2::read::GzDecoder::new(bytes)
                .read_to_end(&mut decoded)
                .expect("gzip"),
            _ => {
                decoded.extend_from_slice(bytes);
                0
            }
        };
        decoded
    }
}

#[tokio::test]
async fn the_server_picks_brotli_then_gzip_then_identity_from_accept_encoding() {
    let site = PublishedSite::build().await;
    let original = site.original(PublishedSite::RUNTIME);
    let matrix = [
        ("br", "br"),
        ("gzip", "gzip"),
        ("gzip, deflate, br", "br"),
        ("br;q=0, gzip", "gzip"),
        ("br;q=0.5, gzip;q=0.8", "gzip"),
        ("br;q=0, gzip;q=0", ""),
        ("identity", ""),
        ("", ""),
    ];
    for (accepted, expected) in matrix {
        let response = site
            .get(
                PublishedSite::RUNTIME,
                &[(ACCEPT_ENCODING.as_str(), accepted)],
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK, "{accepted}");
        assert_eq!(
            PublishedSite::header(&response, &CONTENT_ENCODING),
            expected,
            "{accepted}"
        );
        assert_eq!(
            PublishedSite::header(&response, &VARY),
            "accept-encoding",
            "{accepted}"
        );
        assert!(PublishedSite::header(&response, &CONTENT_TYPE).contains("javascript"));
        let length: usize = PublishedSite::header(&response, &CONTENT_LENGTH)
            .parse()
            .expect("length");
        let body = PublishedSite::body(response).await;
        assert_eq!(body.len(), length, "{accepted}");
        assert_eq!(
            PublishedSite::decoded(expected, &body),
            original,
            "{accepted}"
        );
    }
}

#[tokio::test]
async fn each_variant_has_its_own_etag_and_revalidates() {
    let site = PublishedSite::build().await;
    let mut tags = Vec::new();
    for accepted in ["br", "gzip", "identity"] {
        let response = site
            .get(
                PublishedSite::RUNTIME,
                &[(ACCEPT_ENCODING.as_str(), accepted)],
            )
            .await;
        tags.push(PublishedSite::header(&response, &ETAG).to_owned());
    }
    assert!(tags.iter().all(|tag| !tag.is_empty()));
    assert_ne!(tags[0], tags[1]);
    assert_ne!(tags[1], tags[2]);
    assert_ne!(tags[0], tags[2]);
    let revalidated = site
        .get(
            PublishedSite::RUNTIME,
            &[
                (ACCEPT_ENCODING.as_str(), "br"),
                (IF_NONE_MATCH.as_str(), &tags[0]),
            ],
        )
        .await;
    assert_eq!(revalidated.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn head_and_range_requests_describe_the_chosen_variant() {
    let site = PublishedSite::build().await;
    let brotli = fs::read(site.output.join(".tachyon/tac-client.js.br")).expect("brotli file");
    let head = PublishedSite::send(
        &site.files,
        Method::HEAD,
        PublishedSite::RUNTIME,
        &[(ACCEPT_ENCODING.as_str(), "br")],
    )
    .await;
    assert_eq!(PublishedSite::header(&head, &CONTENT_ENCODING), "br");
    assert_eq!(
        PublishedSite::header(&head, &CONTENT_LENGTH),
        brotli.len().to_string()
    );
    assert!(PublishedSite::body(head).await.is_empty());
    let range = site
        .get(
            PublishedSite::RUNTIME,
            &[
                (ACCEPT_ENCODING.as_str(), "br"),
                (RANGE.as_str(), "bytes=0-9"),
            ],
        )
        .await;
    assert_eq!(range.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(PublishedSite::header(&range, &CONTENT_ENCODING), "br");
    assert_eq!(PublishedSite::body(range).await, brotli[..10]);
}

#[tokio::test]
async fn the_page_and_the_service_worker_are_served_compressed_and_decode_exactly() {
    let site = PublishedSite::build().await;
    for (uri, file) in [("/", "/index.html"), ("/tachyon-sw.js", "/tachyon-sw.js")] {
        let response = site.get(uri, &[(ACCEPT_ENCODING.as_str(), "br")]).await;
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        assert_eq!(
            PublishedSite::header(&response, &CONTENT_ENCODING),
            "br",
            "{uri}"
        );
        let body = PublishedSite::body(response).await;
        assert_eq!(
            PublishedSite::decoded("br", &body),
            site.original(file),
            "{uri}"
        );
    }
    let worker = String::from_utf8(site.original("/tachyon-sw.js")).expect("worker text");
    assert!(!worker.contains(".js.br") && !worker.contains(".js.gz"));
}

#[tokio::test]
async fn a_watching_server_serves_identity_so_live_reload_can_edit_the_page() {
    let site = PublishedSite::build().await;
    assert!(Path::new(&site.output.join("index.html.br")).is_file());
    let response = PublishedSite::send(
        &site.identity,
        Method::GET,
        "/",
        &[(ACCEPT_ENCODING.as_str(), "br, gzip")],
    )
    .await;
    assert_eq!(PublishedSite::header(&response, &CONTENT_ENCODING), "");
    assert_eq!(
        PublishedSite::body(response).await,
        site.original("/index.html")
    );
}
