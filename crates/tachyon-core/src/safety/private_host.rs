use super::{AddressRanges, HostLabel, LocalHostName};
use serde_json::Value;
use std::net::IpAddr;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct PrivateHost {
    host: String,
    networks: AddressRanges,
}

impl PrivateHost {
    const HOST_KEY: &'static str = "host";
    const NETWORKS_KEY: &'static str = "networks";
    const MEMBER_COUNT: usize = 2;
    const PRIVATE_RANGES: [&'static str; 5] = [
        "10.0.0.0/8",
        "172.16.0.0/12",
        "192.168.0.0/16",
        "100.64.0.0/10",
        "fc00::/7",
    ];
    pub(crate) const SHAPE_MESSAGE: &'static str = "yon.outgoing.private must be a list of entries such as {\"host\": \"keycloak\", \"networks\": [\"172.16.0.0/12\"]}, each with only host and networks (ADR 0094).";
    const HOST_MESSAGE: &'static str = "Each yon.outgoing.private host must be one exact host name that is also listed in yon.outgoing.hosts: not a wildcard, an IP address or a localhost name (ADR 0094).";
    const NETWORKS_MESSAGE: &'static str = "Each yon.outgoing.private entry needs at least one network, and every network must be an IP address or CIDR range inside 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, 100.64.0.0/10 or fc00::/7 (ADR 0094).";

    pub(crate) fn read(value: &Value, hosts: &[String]) -> Result<Self, &'static str> {
        let (host, networks) = Self::members(value).ok_or(Self::SHAPE_MESSAGE)?;
        let host = Self::checked_host(host, hosts).ok_or(Self::HOST_MESSAGE)?;
        let networks = Self::checked_networks(&networks).ok_or(Self::NETWORKS_MESSAGE)?;
        Ok(Self { host, networks })
    }

    pub(crate) fn name(&self) -> &str {
        &self.host
    }

    pub(crate) fn admits(&self, host: &str, address: IpAddr) -> bool {
        [self.host == host, self.networks.contains(address)] == [true, true]
    }

    fn members(value: &Value) -> Option<(String, Vec<&str>)> {
        let object = value
            .as_object()
            .filter(|object| object.len() == Self::MEMBER_COUNT)?;
        let host = object.get(Self::HOST_KEY)?.as_str()?.to_ascii_lowercase();
        let networks = object.get(Self::NETWORKS_KEY)?.as_array()?;
        let networks = networks
            .iter()
            .map(Value::as_str)
            .collect::<Option<Vec<_>>>()?;
        Some((host, networks))
    }

    fn checked_host(host: String, hosts: &[String]) -> Option<String> {
        let checks = [
            HostLabel::is_name(&host),
            host.parse::<IpAddr>().is_err(),
            !LocalHostName::is_local(&host),
            hosts.contains(&host),
        ];
        (checks == [true; 4]).then_some(host)
    }

    fn checked_networks(entries: &[&str]) -> Option<AddressRanges> {
        let networks = AddressRanges::parse(entries).filter(|networks| !networks.is_empty())?;
        let private = AddressRanges::parse(&Self::PRIVATE_RANGES)?;
        networks.lies_within(&private).then_some(networks)
    }
}
