use crate::Failure;
use crate::failure::diagnostic;
use std::net::{IpAddr, SocketAddr};
use tokio::net::TcpListener;

pub(in crate::server) struct ServerListener;

impl ServerListener {
    pub(in crate::server) fn validate_exposure(
        host: IpAddr,
        allow_non_loopback: bool,
        server_name: &str,
    ) -> Result<(), Failure> {
        let loopback = host.is_loopback();
        Self::check_exposure(loopback, allow_non_loopback, host, server_name)
    }

    pub(in crate::server) async fn bind(
        host: IpAddr,
        port: u16,
        server_name: &str,
    ) -> Result<(TcpListener, SocketAddr), Failure> {
        let listener = TcpListener::bind(SocketAddr::new(host, port))
            .await
            .map_err(|error| {
                Failure::one(diagnostic(
                    1302,
                    format!("Cannot bind the {server_name} to {host}:{port}: {error}"),
                    Some(String::from("Choose an available interface and port.")),
                    None,
                ))
            })?;
        let address = listener.local_addr().map_err(|error| {
            Failure::one(diagnostic(
                1302,
                format!("Cannot read the {server_name} address: {error}"),
                None,
                None,
            ))
        })?;
        Ok((listener, address))
    }

    fn check_exposure(
        loopback: bool,
        allow_non_loopback: bool,
        host: IpAddr,
        server_name: &str,
    ) -> Result<(), Failure> {
        if loopback || allow_non_loopback {
            return Ok(());
        }
        Err(Failure::one(diagnostic(
            1301,
            format!("Refusing to expose the {server_name} on {host} without explicit permission."),
            Some(String::from(
                "Use a loopback address or pass --allow-non-loopback.",
            )),
            None,
        )))
    }
}
