use std::net::{IpAddr, Ipv4Addr};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewServerOptions {
    pub host: IpAddr,
    pub port: u16,
    pub allow_non_loopback: bool,
}

impl Default for PreviewServerOptions {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 3000,
            allow_non_loopback: false,
        }
    }
}
