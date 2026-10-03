use crate::safety::LocalHostName;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub(super) struct EgressAddress;

impl EgressAddress {
    const THIS_NETWORK_FIRST_OCTET: u8 = 0;
    const SHARED_FIRST_OCTET: u8 = 100;
    const SHARED_SECOND_OCTET_MASK: u8 = 0b1100_0000;
    const SHARED_SECOND_OCTET_PREFIX: u8 = 0b0100_0000;
    const SITE_LOCAL_PREFIX: u16 = 0xfec0;
    const SITE_LOCAL_MASK: u16 = 0xffc0;

    pub(super) async fn candidates(host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>> {
        Self::resolved_unless_fixed(host, port, Self::fixed(host, port)).await
    }

    pub(super) fn is_public(address: IpAddr) -> bool {
        let canonical = address.to_canonical();
        [Self::is_local_v4(canonical), Self::is_local_v6(canonical)] == [false, false]
    }

    fn fixed(host: &str, port: u16) -> Option<Vec<SocketAddr>> {
        let listed = host
            .parse::<IpAddr>()
            .ok()
            .map(|address| vec![SocketAddr::new(address, port)]);
        listed.or_else(|| LocalHostName::is_local(host).then(|| Self::loopback(port)))
    }

    fn loopback(port: u16) -> Vec<SocketAddr> {
        vec![
            SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
        ]
    }

    async fn resolved_unless_fixed(
        host: &str,
        port: u16,
        fixed: Option<Vec<SocketAddr>>,
    ) -> std::io::Result<Vec<SocketAddr>> {
        if let Some(addresses) = fixed {
            return Ok(addresses);
        }
        let resolved = tokio::net::lookup_host((host, port)).await?;
        Ok(resolved
            .filter(|address| Self::is_public(address.ip()))
            .collect())
    }

    fn is_local_v4(address: IpAddr) -> bool {
        let IpAddr::V4(address) = address else {
            return false;
        };
        let octets = address.octets();
        let shared_octets = [octets[0], octets[1] & Self::SHARED_SECOND_OCTET_MASK];
        [
            octets[0] == Self::THIS_NETWORK_FIRST_OCTET,
            shared_octets == [Self::SHARED_FIRST_OCTET, Self::SHARED_SECOND_OCTET_PREFIX],
            address.is_loopback(),
            address.is_private(),
            address.is_link_local(),
            address.is_broadcast(),
        ]
        .contains(&true)
    }

    fn is_local_v6(address: IpAddr) -> bool {
        let IpAddr::V6(address) = address else {
            return false;
        };
        [
            address.segments()[0] & Self::SITE_LOCAL_MASK == Self::SITE_LOCAL_PREFIX,
            address.is_loopback(),
            address.is_unspecified(),
            address.is_unique_local(),
            address.is_unicast_link_local(),
        ]
        .contains(&true)
    }
}
