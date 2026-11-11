use super::AddressRanges;
use std::net::IpAddr;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TrustedProxies {
    ranges: AddressRanges,
}

impl TrustedProxies {
    pub(crate) fn parse(entries: &[&str]) -> Option<Self> {
        let ranges = AddressRanges::parse(entries)?;
        Some(Self { ranges })
    }

    pub(crate) fn contains(&self, address: IpAddr) -> bool {
        self.ranges.contains(address)
    }
}
