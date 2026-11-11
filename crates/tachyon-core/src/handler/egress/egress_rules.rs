use super::{EgressPrivateReport, EgressResolver};
use crate::safety::DeclaredHosts;
use std::net::SocketAddr;
use tokio::net::TcpStream;

#[derive(Debug)]
pub(super) struct EgressRules {
    hosts: DeclaredHosts,
    resolver: EgressResolver,
    report: EgressPrivateReport,
}

impl EgressRules {
    pub(super) fn new(hosts: DeclaredHosts, resolver: EgressResolver) -> Self {
        Self {
            hosts,
            resolver,
            report: EgressPrivateReport::default(),
        }
    }

    pub(super) const fn hosts(&self) -> &DeclaredHosts {
        &self.hosts
    }

    pub(super) async fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>> {
        self.resolver.resolve(host, port).await
    }

    pub(super) fn note_connection(&self, host: &str, stream: &TcpStream) {
        let private = stream
            .peer_addr()
            .ok()
            .filter(|peer| self.hosts.private().admits(host, peer.ip()));
        let _noted = private.map(|peer| self.report.note(host, peer));
    }
}
