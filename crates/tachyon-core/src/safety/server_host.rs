use super::LocalHostName;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerHost {
    address: IpAddr,
    name: Option<String>,
}

impl ServerHost {
    const PLAIN_LOCALHOST: &'static str = "localhost";

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let address = text.parse::<IpAddr>().ok().map(Self::from);
        address.or_else(|| Self::local_name(text))
    }

    #[must_use]
    pub const fn address(&self) -> IpAddr {
        self.address
    }

    #[must_use]
    pub fn setting(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| self.address.to_string())
    }

    #[must_use]
    pub fn url(&self, bound: SocketAddr) -> String {
        let authority = self.name.as_ref().map_or_else(
            || bound.to_string(),
            |name| format!("{name}:{}", bound.port()),
        );
        format!("http://{authority}/")
    }

    fn local_name(text: &str) -> Option<Self> {
        let name = LocalHostName::bind_name(text)?;
        Some(Self {
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            name: (name != Self::PLAIN_LOCALHOST).then_some(name),
        })
    }
}

impl From<IpAddr> for ServerHost {
    fn from(address: IpAddr) -> Self {
        Self {
            address,
            name: None,
        }
    }
}
