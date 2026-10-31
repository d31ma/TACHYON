use std::io;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use super::super::drivers::connection_url;
use super::super::{failure, text};
use super::RedisChannel;
use crate::Failure;

/// Where and how to reach Redis: `redis://host[:port][/db]`, or `rediss://` for TLS checked
/// against the bundled `WebPKI` roots, with the username and password from the settings.
pub(super) struct RedisTarget {
    host: String,
    port: u16,
    tls: bool,
    database: i64,
    username: String,
    password: String,
}

impl RedisTarget {
    const DEFAULT_PORT: u16 = 6379;
    /// Covers the TCP connect, the TLS handshake, `AUTH` and `SELECT`.
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
    /// After a failed attempt, wait this long (plus up to as long again at random) and retry.
    const BACKOFF_MILLISECONDS: [u64; 6] = [100, 200, 400, 800, 1600, 3200];

    pub(super) fn from_settings(settings: &Value) -> Result<Self, Failure> {
        let tls = text(settings, "url")?.starts_with("rediss:");
        let url = connection_url(settings, Self::scheme(tls))?;
        Ok(Self {
            host: Self::host(&url),
            port: url.port().unwrap_or(Self::DEFAULT_PORT),
            tls,
            database: Self::database(&url)?,
            username: Self::credential(settings, "username"),
            password: Self::credential(settings, "password"),
        })
    }

    /// Opens a connection, retrying with exponential backoff and jitter.
    pub(super) async fn connect(&self) -> io::Result<RedisChannel> {
        self.retry().await
    }

    async fn retry(&self) -> io::Result<RedisChannel> {
        let mut delays = Self::BACKOFF_MILLISECONDS.iter();
        let mut result = self.attempt().await;
        while let (Err(_), Some(delay)) = (&result, delays.next()) {
            tokio::time::sleep(Self::jittered(*delay)).await;
            result = self.attempt().await;
        }
        result
    }

    async fn attempt(&self) -> io::Result<RedisChannel> {
        tokio::time::timeout(Self::CONNECT_TIMEOUT, self.open())
            .await
            .map_err(io::Error::other)?
    }

    async fn open(&self) -> io::Result<RedisChannel> {
        let stream = TcpStream::connect((self.host.as_str(), self.port)).await?;
        if !self.tls {
            return self.handshake(RedisChannel::spawn(stream)).await;
        }
        let stream = Self::connector()?
            .connect(self.server_name()?, stream)
            .await?;
        self.handshake(RedisChannel::spawn(stream)).await
    }

    async fn handshake(&self, channel: RedisChannel) -> io::Result<RedisChannel> {
        let database = self.database.to_string();
        let setup = [self.authentication(), Self::selection(&database)].concat();
        for command in setup {
            channel
                .call(&command)
                .await
                .ok_or_else(|| io::Error::other("Redis refused the connection setup"))?;
        }
        Ok(channel)
    }

    /// `AUTH` only with a password, naming the user when there is one.
    fn authentication(&self) -> Vec<Vec<&[u8]>> {
        match (self.username.is_empty(), self.password.is_empty()) {
            (_, true) => Vec::new(),
            (true, false) => vec![vec![b"AUTH", self.password.as_bytes()]],
            (false, false) => vec![vec![
                b"AUTH",
                self.username.as_bytes(),
                self.password.as_bytes(),
            ]],
        }
    }

    fn selection(database: &str) -> Vec<Vec<&[u8]>> {
        if database == "0" {
            return Vec::new();
        }
        vec![vec![b"SELECT", database.as_bytes()]]
    }

    fn connector() -> io::Result<TlsConnector> {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(io::Error::other)?
                .with_root_certificates(roots)
                .with_no_client_auth();
        Ok(TlsConnector::from(Arc::new(config)))
    }

    fn server_name(&self) -> io::Result<ServerName<'static>> {
        ServerName::try_from(self.host.clone()).map_err(io::Error::other)
    }

    fn jittered(milliseconds: u64) -> Duration {
        let jitter = getrandom::u64().unwrap_or_default() % milliseconds;
        Duration::from_millis(milliseconds + jitter)
    }

    fn scheme(tls: bool) -> &'static str {
        if tls { "rediss" } else { "redis" }
    }

    /// An IPv6 host keeps no brackets, so it reads as an address for both TCP and TLS.
    fn host(url: &url::Url) -> String {
        url.host_str()
            .unwrap_or_default()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned()
    }

    fn database(url: &url::Url) -> Result<i64, Failure> {
        match url.path().trim_matches('/') {
            "" => Ok(0),
            path => path
                .parse()
                .map_err(|_| failure("Invalid Redis connection configuration.")),
        }
    }

    /// Empty when unset, as in a URL without that credential.
    fn credential(settings: &Value, key: &str) -> String {
        settings[key].as_str().unwrap_or_default().to_owned()
    }
}
