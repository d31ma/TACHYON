struct Middleware;
impl Middleware {
    fn before(request: &http::Request<Vec<u8>>) -> Option<http::Response<Vec<u8>>> {
        let token = request.headers().get("authorization").and_then(|value| value.to_str().ok());
        if !request.uri().path().starts_with("/private") || token == Some("let-me-in") {
            return None;
        }
        http::Response::builder()
            .status(401)
            .header("content-type", "application/json")
            .body(b"{\"error\":\"Unauthorized\"}".to_vec())
            .ok()
    }

    fn after(_request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
        http::Response::builder()
            .status(204)
            .header("x-served-by", "tachyon")
            .body(Vec::new())
            .unwrap_or_default()
    }
}
