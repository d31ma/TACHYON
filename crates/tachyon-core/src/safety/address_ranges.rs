use std::net::IpAddr;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AddressRanges {
    ranges: Vec<(IpAddr, u8)>,
}

impl AddressRanges {
    const SEPARATOR: char = '/';
    const IPV4_BITS: u8 = 32;
    const IPV6_BITS: u8 = 128;

    pub(crate) fn parse(entries: &[&str]) -> Option<Self> {
        let ranges = entries
            .iter()
            .map(|entry| Self::range(entry))
            .collect::<Option<Vec<_>>>()?;
        Some(Self { ranges })
    }

    pub(crate) fn contains(&self, address: IpAddr) -> bool {
        let address = address.to_canonical();
        self.ranges
            .iter()
            .any(|(network, prefix)| Self::within(address, *network, *prefix))
    }

    pub(crate) fn lies_within(&self, outer: &Self) -> bool {
        self.ranges.iter().all(|range| outer.covers(*range))
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    fn covers(&self, inner: (IpAddr, u8)) -> bool {
        self.ranges.iter().any(|outer| Self::holds(*outer, inner))
    }

    fn holds(outer: (IpAddr, u8), inner: (IpAddr, u8)) -> bool {
        let (network, prefix) = outer;
        let (address, length) = inner;
        [length >= prefix, Self::within(address, network, prefix)] == [true, true]
    }

    fn range(entry: &str) -> Option<(IpAddr, u8)> {
        let (address, prefix) = entry
            .split_once(Self::SEPARATOR)
            .map_or((entry, None), |(address, prefix)| (address, Some(prefix)));
        let address = address.parse::<IpAddr>().ok()?.to_canonical();
        let bits = Self::bits(address);
        let prefix = prefix.map_or(Some(bits), |prefix| prefix.parse::<u8>().ok())?;
        (prefix <= bits).then_some((address, prefix))
    }

    const fn bits(address: IpAddr) -> u8 {
        match address {
            IpAddr::V4(_) => Self::IPV4_BITS,
            IpAddr::V6(_) => Self::IPV6_BITS,
        }
    }

    fn number(address: IpAddr) -> u128 {
        match address {
            IpAddr::V4(address) => u128::from(u32::from(address)),
            IpAddr::V6(address) => u128::from(address),
        }
    }

    fn within(address: IpAddr, network: IpAddr, prefix: u8) -> bool {
        let bits = Self::bits(network);
        let mask = u128::MAX.checked_shl(u32::from(bits - prefix)).unwrap_or(0);
        let same_family = Self::bits(address) == bits;
        let same_network = Self::number(address) & mask == Self::number(network) & mask;
        [same_family, same_network] == [true, true]
    }
}
