use super::ServerResponse;
use crate::safety::{DeclaredHosts, LocalHostName};
use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::HOST;
use axum::http::uri::Authority;
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode};
use axum::middleware::Next;
use cap_std::fs::Dir;
use serde_json::Value;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(in crate::server) struct HostGuard {
    names: Arc<DeclaredHosts>,
    origin: Option<String>,
    checks: bool,
}

impl HostGuard {
    const MANIFEST: &'static str = "client/manifest.json";
    const PUBLISHED_MANIFEST: &'static str = "manifest.webmanifest";
    const ORIGIN_POINTER: &'static str = "/tac/origin";
    const TRAILING_DOT: char = '.';
    const USER_INFO_MARK: char = '@';
    const MALFORMED: &'static str = "Tachyon cannot read the Host header of this request.";
    const SETTING: &'static str = "server.hosts in server/application.json (yon.server.hosts)";

    pub(in crate::server) fn new(bind: IpAddr, hosts: &[String], origin: Option<String>) -> Self {
        let names = DeclaredHosts::named(hosts, Self::SETTING);
        let checks = [bind.is_loopback(), !names.is_empty()].contains(&true);
        Self {
            names: Arc::new(names),
            origin,
            checks,
        }
    }

    pub(in crate::server) fn for_project(bind: IpAddr, project: &Dir, hosts: &[String]) -> Self {
        Self::new(bind, hosts, Self::site_origin(project, Self::MANIFEST))
    }

    pub(in crate::server) fn for_bundle(bind: IpAddr, bundle: &Path, hosts: &[String]) -> Self {
        let directory = Dir::open_ambient_dir(bundle, cap_std::ambient_authority()).ok();
        let origin =
            directory.and_then(|directory| Self::site_origin(&directory, Self::PUBLISHED_MANIFEST));
        Self::new(bind, hosts, origin)
    }

    pub(in crate::server) fn protect(self, application: Router) -> Router {
        application.layer(axum::middleware::from_fn_with_state(self, Self::check))
    }

    async fn check(State(guard): State<Self>, request: Request, next: Next) -> Response<Body> {
        match guard.refusal_for(request.headers()) {
            None => next.run(request).await,
            Some(refusal) => refusal,
        }
    }

    pub(in crate::server) fn refusal_for(&self, headers: &HeaderMap) -> Option<Response<Body>> {
        let named = self.checks.then(|| Self::named_host(headers)).flatten()?;
        named.map_or_else(|()| Some(Self::malformed()), |name| self.verdict(&name))
    }

    fn named_host(headers: &HeaderMap) -> Option<Result<String, ()>> {
        let mut values = headers.get_all(HOST).iter();
        let value = values.next()?;
        let single = values.next().is_none();
        let blank_single = [single, value.is_empty()] == [true, true];
        let named = Self::host_name(value).filter(|_| single).ok_or(());
        (!blank_single).then_some(named)
    }

    fn verdict(&self, name: &str) -> Option<Response<Body>> {
        (!self.allows(name)).then(|| Self::refusal(name))
    }

    fn host_name(value: &HeaderValue) -> Option<String> {
        let text = value.to_str().ok()?;
        let authority = text
            .parse::<Authority>()
            .ok()
            .filter(|authority| !authority.as_str().contains(Self::USER_INFO_MARK))?;
        let host = authority
            .host()
            .trim_start_matches('[')
            .trim_end_matches(']');
        Some(host.to_ascii_lowercase())
    }

    fn allows(&self, name: &str) -> bool {
        let address = name.parse::<IpAddr>().is_ok();
        address || LocalHostName::is_local(name) || self.is_origin(name) || self.names.allows(name)
    }

    fn is_origin(&self, name: &str) -> bool {
        let name = name.strip_suffix(Self::TRAILING_DOT).unwrap_or(name);
        self.origin.as_deref() == Some(name)
    }

    fn site_origin(directory: &Dir, file: &str) -> Option<String> {
        let bytes = directory.read(file).ok()?;
        let document: Value = serde_json::from_slice(&bytes).ok()?;
        let origin = document.pointer(Self::ORIGIN_POINTER)?.as_str()?;
        let address = url::Url::parse(origin).ok()?;
        address.host_str().map(str::to_ascii_lowercase)
    }

    fn refusal(name: &str) -> Response<Body> {
        let message = format!(
            "TY1366: Tachyon refused this request because its Host name {name} is not a localhost name, an IP address, the tac.origin host or a name in server.hosts. To allow it, add the name to server.hosts in server/application.json (ADR 0094)."
        );
        ServerResponse::text(StatusCode::FORBIDDEN, &message)
    }

    fn malformed() -> Response<Body> {
        ServerResponse::text(StatusCode::BAD_REQUEST, Self::MALFORMED)
    }
}
