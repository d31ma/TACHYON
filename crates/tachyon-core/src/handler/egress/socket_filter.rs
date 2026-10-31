use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
    SeccompRule, TargetArch,
};
use std::collections::BTreeMap;

pub(super) struct SocketFilter;

impl SocketFilter {
    const FAMILY: u8 = 0;
    const KIND: u8 = 1;
    const PROTOCOL: u8 = 2;
    const KIND_MASK: u64 = 0xf;
    const STREAM: u64 = libc::SOCK_STREAM as u64;
    const TCP: u64 = libc::IPPROTO_TCP as u64;
    const ANY_PROTOCOL: u64 = 0;
    const X32_BIT: i64 = 0x4000_0000;
    const NET_FAMILIES: [u64; 2] = [libc::AF_INET as u64, libc::AF_INET6 as u64];
    const LOCAL_FAMILY: u64 = libc::AF_NETLINK as u64;
    const REFUSED: u32 = libc::EACCES as u32;

    pub(super) fn program() -> Result<BpfProgram, String> {
        let arch =
            TargetArch::try_from(std::env::consts::ARCH).map_err(|error| error.to_string())?;
        let filter = SeccompFilter::new(
            Self::rules()?,
            SeccompAction::Allow,
            SeccompAction::Errno(Self::REFUSED),
            arch,
        )
        .map_err(|error| error.to_string())?;
        BpfProgram::try_from(filter).map_err(|error| error.to_string())
    }

    pub(super) fn apply(program: &BpfProgram) -> Result<(), String> {
        seccompiler::apply_filter(program).map_err(|error| error.to_string())
    }

    fn rules() -> Result<BTreeMap<i64, Vec<SeccompRule>>, String> {
        let mut rules = BTreeMap::from([
            (libc::SYS_socket, Self::socket_rules()?),
            (libc::SYS_io_uring_setup, Vec::new()),
        ]);
        if cfg!(target_arch = "x86_64") {
            rules.insert(libc::SYS_socket | Self::X32_BIT, Vec::new());
            rules.insert(libc::SYS_io_uring_setup | Self::X32_BIT, Vec::new());
        }
        Ok(rules)
    }

    fn socket_rules() -> Result<Vec<SeccompRule>, String> {
        let mut rules = vec![Self::rule(vec![
            Self::condition(Self::FAMILY, SeccompCmpOp::Ne, Self::NET_FAMILIES[0])?,
            Self::condition(Self::FAMILY, SeccompCmpOp::Ne, Self::NET_FAMILIES[1])?,
            Self::condition(Self::FAMILY, SeccompCmpOp::Ne, Self::LOCAL_FAMILY)?,
        ])?];
        for family in Self::NET_FAMILIES {
            for kind in (0..=Self::KIND_MASK).filter(|kind| *kind != Self::STREAM) {
                rules.push(Self::rule(vec![
                    Self::condition(Self::FAMILY, SeccompCmpOp::Eq, family)?,
                    Self::condition(Self::KIND, SeccompCmpOp::MaskedEq(Self::KIND_MASK), kind)?,
                ])?);
            }
            rules.push(Self::rule(vec![
                Self::condition(Self::FAMILY, SeccompCmpOp::Eq, family)?,
                Self::condition(Self::PROTOCOL, SeccompCmpOp::Ne, Self::ANY_PROTOCOL)?,
                Self::condition(Self::PROTOCOL, SeccompCmpOp::Ne, Self::TCP)?,
            ])?);
        }
        Ok(rules)
    }

    fn rule(conditions: Vec<SeccompCondition>) -> Result<SeccompRule, String> {
        SeccompRule::new(conditions).map_err(|error| error.to_string())
    }

    fn condition(
        argument: u8,
        operator: SeccompCmpOp,
        value: u64,
    ) -> Result<SeccompCondition, String> {
        SeccompCondition::new(argument, SeccompCmpArgLen::Dword, operator, value)
            .map_err(|error| error.to_string())
    }
}
