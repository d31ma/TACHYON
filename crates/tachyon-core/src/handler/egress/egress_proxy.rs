use super::{EgressResolver, EgressRules, EgressTunnel};
use crate::Failure;
use crate::failure::diagnostic;
use crate::safety::DeclaredHosts;
use crate::sorted_json::SortedJson;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub(crate) struct EgressProxy;

impl EgressProxy {
    const LOOPBACK: &'static str = "127.0.0.1";
    const PROXY_NAMES: [&'static str; 6] = [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ];
    const BYPASS_NAMES: [&'static str; 2] = ["NO_PROXY", "no_proxy"];
    const NODE_PROXY_SWITCH: &'static str = "NODE_USE_ENV_PROXY";
    const GUARD_ADDRESS_NAME: &'static str = "YON_OUTGOING_PROXY";
    const MAX_CONNECTIONS: usize = 256;
    const ACCEPT_PAUSE: Duration = Duration::from_millis(100);

    pub(crate) async fn start(hosts: DeclaredHosts) -> Result<SocketAddr, Failure> {
        Self::start_with(EgressRules::new(hosts, EgressResolver::system())).await
    }

    pub(super) async fn start_with(rules: EgressRules) -> Result<SocketAddr, Failure> {
        let listener = TcpListener::bind((Self::LOOPBACK, 0))
            .await
            .map_err(|error| Self::failure(&error))?;
        let address = listener
            .local_addr()
            .map_err(|error| Self::failure(&error))?;
        tokio::spawn(Self::accept(listener, Arc::new(rules)));
        Ok(address)
    }

    pub(crate) fn apply(url: &str, command: &mut Command) {
        Self::set_proxy_names(url, command);
        Self::remove_bypasses(command);
        command.env(Self::NODE_PROXY_SWITCH, "1");
        command.env(Self::GUARD_ADDRESS_NAME, url);
    }

    fn remove_bypasses(command: &mut Command) {
        for name in Self::BYPASS_NAMES {
            command.env_remove(name);
        }
    }

    fn set_proxy_names(url: &str, command: &mut Command) {
        for name in Self::PROXY_NAMES {
            command.env(name, url);
        }
    }

    async fn accept(listener: TcpListener, rules: Arc<EgressRules>) {
        let slots = Arc::new(Semaphore::new(Self::MAX_CONNECTIONS));
        let mut reported = HashSet::new();
        loop {
            Self::take(listener.accept().await, &rules, &slots, &mut reported).await;
        }
    }

    async fn take(
        accepted: std::io::Result<(TcpStream, SocketAddr)>,
        rules: &Arc<EgressRules>,
        slots: &Arc<Semaphore>,
        reported: &mut HashSet<ErrorKind>,
    ) {
        match accepted {
            Ok((stream, _)) => Self::admit(stream, rules, slots),
            Err(error) => Self::pause_after(&error, reported).await,
        }
    }

    fn admit(stream: TcpStream, rules: &Arc<EgressRules>, slots: &Arc<Semaphore>) {
        let slot = Arc::clone(slots).try_acquire_owned().ok();
        tokio::spawn(Self::serve_or_refuse(stream, Arc::clone(rules), slot));
    }

    async fn serve_or_refuse(
        stream: TcpStream,
        rules: Arc<EgressRules>,
        slot: Option<OwnedSemaphorePermit>,
    ) {
        match slot {
            Some(slot) => {
                EgressTunnel::serve(stream, rules).await;
                drop(slot);
            }
            None => EgressTunnel::refuse_busy(stream).await,
        }
    }

    async fn pause_after(error: &std::io::Error, reported: &mut HashSet<ErrorKind>) {
        let _reported = reported
            .insert(error.kind())
            .then(|| Self::report_accept_failure(error));
        tokio::time::sleep(Self::ACCEPT_PAUSE).await;
    }

    fn report_accept_failure(error: &std::io::Error) {
        eprintln!(
            "{}",
            SortedJson::text(&serde_json::json!({
                "event": "handler.egress_accept_failed",
                "reason": error.kind().to_string(),
            }))
        );
    }

    fn failure(error: &std::io::Error) -> Failure {
        Failure::one(diagnostic(
            1326,
            format!("Cannot start the outgoing request guard: {error}"),
            Some(String::from(
                "Tachyon sends Yon's outgoing HTTP requests through a local guard that allows only the hosts in outgoing.hosts (ADR 0068).",
            )),
            None,
        ))
    }
}
