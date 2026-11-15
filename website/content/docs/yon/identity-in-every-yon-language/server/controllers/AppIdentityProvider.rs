use crate::models::Identity;

#[IdentityProvider]
struct AppIdentityProvider {}
impl AppIdentityProvider {
    fn new() -> Self {
        Self {}
    }

    fn identify(&self, request: &http::Request<Vec<u8>>) -> Option<Identity> {
        let token = request.headers().get("authorization");
        Self::find(token.map(http::HeaderValue::as_bytes).unwrap_or_default())
    }

    fn find(token: &[u8]) -> Option<Identity> {
        match token {
            b"ada-token" => Some(Identity { user: String::from("ada"), roles: vec![String::from("Admin")] }),
            b"bob-token" => Some(Identity { user: String::from("bob"), roles: Vec::new() }),
            _ => None,
        }
    }
}
