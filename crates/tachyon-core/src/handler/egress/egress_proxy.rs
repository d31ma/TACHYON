use super::EgressTunnel;
use crate::Failure;
use crate::failure::diagnostic;
use crate::safety::DeclaredHosts;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::process::Command;

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

    pub(crate) async fn start(hosts: DeclaredHosts) -> Result<String, Failure> {
        let listener = TcpListener::bind((Self::LOOPBACK, 0))
            .await
            .map_err(|error| Self::failure(&error))?;
        let address = listener
            .local_addr()
            .map_err(|error| Self::failure(&error))?;
        tokio::spawn(Self::accept(listener, Arc::new(hosts)));
        Ok(format!("http://{address}"))
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
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(EgressTunnel::serve(stream, Arc::clone(&hosts)));
        }
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
