use super::PublicAssets;
use axum::body::Body;
use axum::http::header::{AUTHORIZATION, CACHE_CONTROL, COOKIE, HeaderName};
use axum::http::{HeaderMap, HeaderValue, Request, Response, StatusCode, Uri};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Clone, Debug)]
pub(in crate::server) struct AssetCachePolicy {
    root: Option<PathBuf>,
    assets: Arc<Mutex<PublicAssets>>,
}

impl AssetCachePolicy {
    const REVALIDATE: &'static str = "no-cache";
    const STORED_STATUSES: [StatusCode; 2] = [StatusCode::OK, StatusCode::NOT_MODIFIED];
    const CREDENTIAL_HEADERS: [HeaderName; 2] = [AUTHORIZATION, COOKIE];

    pub(in crate::server) fn new(root: Option<&Path>) -> Self {
        let assets = root.map(PublicAssets::read).unwrap_or_default();
        Self {
            root: root.map(Path::to_path_buf),
            assets: Arc::new(Mutex::new(assets)),
        }
    }

    pub(in crate::server) fn is_public_asset(&self, request: &Request<Body>) -> bool {
        let anonymous = Self::is_anonymous(request.headers());
        let listed = self.lists(request.uri());
        [anonymous, listed] == [true, true]
    }

    pub(in crate::server) fn mark(public: bool, mut response: Response<Body>) -> Response<Body> {
        let stored = Self::STORED_STATUSES.contains(&response.status());
        let revalidate = HeaderValue::from_static(Self::REVALIDATE);
        let header = [public, stored] == [true, true];
        response
            .headers_mut()
            .extend(header.then_some((CACHE_CONTROL, revalidate)));
        response
    }

    fn is_anonymous(headers: &HeaderMap) -> bool {
        !Self::CREDENTIAL_HEADERS
            .iter()
            .any(|name| headers.contains_key(name))
    }

    fn lists(&self, uri: &Uri) -> bool {
        if uri.query().is_some() {
            return false;
        }
        self.lists_path(uri.path())
    }

    fn lists_path(&self, path: &str) -> bool {
        self.root
            .as_deref()
            .is_some_and(|root| self.lists_under(root, path))
    }

    fn lists_under(&self, root: &Path, path: &str) -> bool {
        let mut assets = self.assets.lock().unwrap_or_else(PoisonError::into_inner);
        Self::refresh(&mut assets, root);
        assets.lists(path)
    }

    fn refresh(assets: &mut PublicAssets, root: &Path) {
        if !assets.is_current(root) {
            *assets = PublicAssets::read(root);
        }
    }
}
