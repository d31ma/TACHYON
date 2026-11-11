use super::{AddressRanges, PrivateHost};
use serde_json::Value;
use std::collections::HashSet;
use std::net::IpAddr;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct PrivateHosts {
    entries: Vec<PrivateHost>,
}

impl PrivateHosts {
    const DUPLICATE_MESSAGE: &'static str = "yon.outgoing.private names a host twice; list all of its networks in one entry (ADR 0094).";
    const METADATA_ADDRESSES: [&'static str; 2] = ["100.100.100.200", "fd00:ec2::254"];
    const TRAILING_DOT: char = '.';

    pub(crate) fn parse(value: Option<&Value>, hosts: &[String]) -> Result<Self, &'static str> {
        let listed = Self::listed(value).ok_or(PrivateHost::SHAPE_MESSAGE)?;
        let entries = listed
            .iter()
            .map(|entry| PrivateHost::read(entry, hosts))
            .collect::<Result<Vec<_>, _>>()?;
        Self::distinct(entries)
    }

    pub(crate) fn admits(&self, host: &str, address: IpAddr) -> bool {
        let host = host
            .trim_end_matches(Self::TRAILING_DOT)
            .to_ascii_lowercase();
        let address = address.to_canonical();
        let declared = self.declared(&host, address);
        [declared, Self::is_metadata(address)] == [true, false]
    }

    fn listed(value: Option<&Value>) -> Option<&[Value]> {
        value.map_or(Some(&[][..]), |value| value.as_array().map(Vec::as_slice))
    }

    fn distinct(entries: Vec<PrivateHost>) -> Result<Self, &'static str> {
        let names = entries
            .iter()
            .map(PrivateHost::name)
            .collect::<HashSet<_>>()
            .len();
        (names == entries.len())
            .then_some(Self { entries })
            .ok_or(Self::DUPLICATE_MESSAGE)
    }

    fn declared(&self, host: &str, address: IpAddr) -> bool {
        self.entries.iter().any(|entry| entry.admits(host, address))
    }

    fn is_metadata(address: IpAddr) -> bool {
        AddressRanges::parse(&Self::METADATA_ADDRESSES)
            .is_none_or(|metadata| metadata.contains(address))
    }
}
