use super::EgressTunnel;
use crate::Failure;
use crate::failure::diagnostic;
use crate::safety::DeclaredHosts;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::Semaphore;

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
        let listener = TcpListener::bind((Self::LOOPBACK, 0))
            .await
            .map_err(|error| Self::failure(&error))?;
        let address = listener
            .local_addr()
            .map_err(|error| Self::failure(&error))?;
        tokio::spawn(Self::accept(listener, Arc::new(hosts)));
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

    async fn accept(listener: TcpListener, hosts: Arc<DeclaredHosts>) {
        let slots = Arc::new(Semaphore::new(Self::MAX_CONNECTIONS));
        let mut reported = HashSet::new();
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let Ok(slot) = Arc::clone(&slots).try_acquire_owned() else {
                        continue;
                    };
                    let hosts = Arc::clone(&hosts);
                    tokio::spawn(async move {
                        EgressTunnel::serve(stream, hosts).await;
                        drop(slot);
                    });
                }
                Err(error) => Self::pause_after(&error, &mut reported).await,
            }
        }
    }

    async fn pause_after(error: &std::io::Error, reported: &mut HashSet<ErrorKind>) {
        if reported.insert(error.kind()) {
            eprintln!(
                "{}",
                serde_json::json!({
                    "event": "handler.egress_accept_failed",
                    "reason": error.kind().to_string(),
                })
            );
        }
        tokio::time::sleep(Self::ACCEPT_PAUSE).await;
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
