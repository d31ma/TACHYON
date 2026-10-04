use crate::safety::TrustedProxies;
use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{FORWARDED, HeaderName};
use axum::http::{HeaderMap, HeaderValue, Response};
use axum::middleware::Next;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(in crate::server) struct ForwardedHeaders {
    trusted: Arc<TrustedProxies>,
}

impl ForwardedHeaders {
    const FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
    const FORWARDED_PROTO: HeaderName = HeaderName::from_static("x-forwarded-proto");
    const REAL_IP: HeaderName = HeaderName::from_static("x-real-ip");
    const UNTRUSTED: [HeaderName; 7] = [
        FORWARDED,
        Self::FORWARDED_FOR,
        HeaderName::from_static("x-forwarded-host"),
        Self::FORWARDED_PROTO,
        HeaderName::from_static("x-forwarded-port"),
        HeaderName::from_static("x-forwarded-prefix"),
        Self::REAL_IP,
    ];
    const LIST_SEPARATOR: char = ',';
    const JOINED_SEPARATOR: &'static str = ", ";
    const SCHEME: &'static str = "http";

    pub(in crate::server) fn new(trusted: TrustedProxies) -> Self {
        Self {
            trusted: Arc::new(trusted),
        }
    }

    pub(in crate::server) fn protect(self, application: Router) -> Router {
        application.layer(axum::middleware::from_fn_with_state(self, Self::rewrite))
    }

    async fn rewrite(State(edge): State<Self>, mut request: Request, next: Next) -> Response<Body> {
        let peer = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(peer)| peer.ip().to_canonical());
        edge.apply(request.headers_mut(), peer);
        next.run(request).await
    }

    pub(in crate::server) fn apply(&self, headers: &mut HeaderMap, peer: Option<IpAddr>) {
        let trusted = peer.is_some_and(|peer| self.trusted.contains(peer));
        Self::remove_untrusted(headers, !trusted);
        let hop = peer
            .map(|peer| self.hop_headers(headers, peer))
            .unwrap_or_default();
        Self::replace(headers, hop);
    }

    fn remove_untrusted(headers: &mut HeaderMap, untrusted: bool) {
        for name in Self::UNTRUSTED.iter().filter(|_| untrusted) {
            headers.remove(name);
        }
    }

    fn hop_headers(&self, headers: &HeaderMap, peer: IpAddr) -> Vec<(HeaderName, String)> {
        let chain = Self::joined(headers, &Self::FORWARDED_FOR, &peer.to_string());
        let client = self.client(&chain, peer);
        let forwarded = Self::joined(headers, &FORWARDED, &Self::forwarded_node(peer));
        let proto = headers
            .get(Self::FORWARDED_PROTO)
            .and_then(|value| value.to_str().ok())
            .unwrap_or(Self::SCHEME)
            .to_owned();
        vec![
            (Self::FORWARDED_FOR, chain),
            (FORWARDED, forwarded),
            (Self::REAL_IP, client.to_string()),
            (Self::FORWARDED_PROTO, proto),
        ]
    }

    fn replace(headers: &mut HeaderMap, values: Vec<(HeaderName, String)>) {
        for (name, value) in values {
            headers.remove(&name);
            headers.extend(
                HeaderValue::from_str(&value)
                    .into_iter()
                    .map(|value| (name.clone(), value)),
            );
        }
    }

    fn client(&self, chain: &str, peer: IpAddr) -> IpAddr {
        let hops = chain
            .rsplit(Self::LIST_SEPARATOR)
            .map_while(|hop| hop.trim().parse::<IpAddr>().ok())
            .map(|hop| hop.to_canonical())
            .collect::<Vec<_>>();
        let farthest = hops.last().copied().unwrap_or(peer);
        hops.into_iter()
            .find(|hop| !self.trusted.contains(*hop))
            .unwrap_or(farthest)
    }

    fn joined(headers: &HeaderMap, name: &HeaderName, added: &str) -> String {
        let earlier = headers
            .get_all(name)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        earlier
            .chain([added])
            .collect::<Vec<_>>()
            .join(Self::JOINED_SEPARATOR)
    }

    fn forwarded_node(peer: IpAddr) -> String {
        match peer {
            IpAddr::V4(address) => format!("for={address};proto={}", Self::SCHEME),
            IpAddr::V6(address) => format!("for=\"[{address}]\";proto={}", Self::SCHEME),
        }
    }
}
