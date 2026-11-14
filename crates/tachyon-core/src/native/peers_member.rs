use super::config::config_failure;
use crate::Failure;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct PeersMember {
    #[serde(default)]
    ice_servers: Vec<String>,
    #[serde(default)]
    relay_only: bool,
}

impl PeersMember {
    pub(crate) const GRANT: &'static str = "peers";
    const META_NAME: &'static str = "tachyon-peers";
    const MOST_SERVERS: usize = 8;
    const MOST_URL_LENGTH: usize = 256;
    const STUN: &'static str = "stun:";
    const TRANSPORT: &'static str = "?transport=";
    const NO_TRANSPORT: &'static str = "";
    const SCHEMES: [(&'static str, &'static [&'static str]); 3] = [
        ("turns:", &["", "tcp"]),
        ("turn:", &["", "udp", "tcp"]),
        ("stun:", &[""]),
    ];
    const NOT_GRANTED_MESSAGE: &'static str = "Manifest tac.peers needs \"peers\" in tac.browserApis, because only a page that grants peers may open peer connections (ADR 0065).";
    const COUNT_MESSAGE: &'static str =
        "Manifest tac.peers.iceServers lists at most 8 STUN or TURN servers.";
    const RELAY_MESSAGE: &'static str = "Manifest tac.peers.relayOnly needs at least one turn: or turns: server in tac.peers.iceServers; without one no connection can start.";
    const URL_MESSAGE: &'static str = "Each entry of manifest tac.peers.iceServers is stun:host:port, turn:host:port, turn:host:port?transport=udp or tcp, or turns:host:port?transport=tcp, such as \"turn:turn.example.com:3478?transport=udp\". There are no default servers.";

    pub(crate) fn checked(
        member: Option<Self>,
        browser_apis: &[String],
        outgoing: Option<&serde_json::Value>,
    ) -> Result<Self, Failure> {
        let granted = browser_apis.iter().any(|grant| grant == Self::GRANT);
        let member = Self::granted_member(member, granted)?;
        member.validate(outgoing)?;
        Ok(member)
    }

    fn granted_member(member: Option<Self>, granted: bool) -> Result<Self, Failure> {
        match (member, granted) {
            (Some(_), false) => Err(config_failure(Self::NOT_GRANTED_MESSAGE)),
            (member, _) => Ok(member.unwrap_or_default()),
        }
    }

    fn validate(&self, outgoing: Option<&serde_json::Value>) -> Result<(), Failure> {
        Self::check_count(self.ice_servers.len())?;
        let hosts = super::yon_member::YonMember::declared_hosts(outgoing);
        self.ice_servers
            .iter()
            .try_for_each(|url| Self::check_server(url, &hosts))?;
        self.check_relay()
    }

    fn check_count(count: usize) -> Result<(), Failure> {
        (count <= Self::MOST_SERVERS)
            .then_some(())
            .ok_or_else(|| config_failure(Self::COUNT_MESSAGE))
    }

    fn check_relay(&self) -> Result<(), Failure> {
        let has_relay = self.ice_servers.iter().any(|url| Self::is_relay(url));
        let refused = [self.relay_only, has_relay] == [true, false];
        (!refused)
            .then_some(())
            .ok_or_else(|| config_failure(Self::RELAY_MESSAGE))
    }

    fn is_relay(url: &str) -> bool {
        !url.starts_with(Self::STUN)
    }

    fn check_server(url: &str, hosts: &crate::safety::DeclaredHosts) -> Result<(), Failure> {
        let host = Self::host(url).ok_or_else(|| config_failure(Self::URL_MESSAGE))?;
        Self::check_declared_host(url, &host, hosts)
    }

    pub(crate) fn host(url: &str) -> Option<String> {
        let written = url.len() <= Self::MOST_URL_LENGTH;
        let (transports, rest) = Self::split_scheme(url).filter(|_| written)?;
        let address = Self::address(transports, rest)?;
        Self::host_of_address(address)
    }

    fn split_scheme(url: &str) -> Option<(&'static [&'static str], &str)> {
        Self::SCHEMES.into_iter().find_map(|(scheme, transports)| {
            url.strip_prefix(scheme).map(|rest| (transports, rest))
        })
    }

    fn address<'a>(transports: &[&str], rest: &'a str) -> Option<&'a str> {
        let (address, transport) = rest
            .split_once(Self::TRANSPORT)
            .unwrap_or((rest, Self::NO_TRANSPORT));
        transports.contains(&transport).then_some(address)
    }

    fn host_of_address(address: &str) -> Option<String> {
        let (written, port) = address.rsplit_once(':')?;
        let port_ok = port.parse::<u16>().is_ok_and(|port| port > 0);
        let bracketed = [!written.contains(':'), written.starts_with('[')].contains(&true);
        let host = written.trim_start_matches('[').trim_end_matches(']');
        let host_ok = Self::is_host(host);
        ([port_ok, bracketed, host_ok] == [true, true, true]).then(|| host.to_ascii_lowercase())
    }

    fn is_host(host: &str) -> bool {
        let allowed = |letter: char| letter.is_ascii_alphanumeric() || ".-:".contains(letter);
        [!host.is_empty(), host.chars().all(allowed)] == [true, true]
    }

    fn check_declared_host(
        url: &str,
        host: &str,
        hosts: &crate::safety::DeclaredHosts,
    ) -> Result<(), Failure> {
        let allowed = [Self::is_local(host), hosts.allows(host)].contains(&true);
        allowed.then_some(()).ok_or_else(|| {
            config_failure(&format!(
                "Manifest tac.peers.iceServers has {url}, but its host {host} is not in tac.outgoing.hosts. A Tac page reaches only declared hosts: add \"{host}\" to tac.outgoing.hosts in client/manifest.json."
            ))
        })
    }

    fn is_local(host: &str) -> bool {
        host.parse::<std::net::IpAddr>().map_or_else(
            |_| crate::safety::LocalHostName::is_local(host),
            |address| address.is_loopback(),
        )
    }

    pub(crate) fn meta_tag(&self) -> String {
        let written = serde_json::to_string(self).unwrap_or_default();
        format!(
            "<meta name=\"{}\" content=\"{}\">",
            Self::META_NAME,
            crate::compiler::html_attribute_escape(&written)
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::PeersMember;

    fn member(servers: &[&str], relay_only: bool) -> PeersMember {
        PeersMember {
            ice_servers: servers.iter().map(|url| String::from(*url)).collect(),
            relay_only,
        }
    }

    fn granted() -> Vec<String> {
        vec![String::from("peers")]
    }

    fn outgoing() -> serde_json::Value {
        serde_json::json!({"hosts": ["turn.example.com", "stun.example.com"]})
    }

    fn checked(servers: &[&str], relay_only: bool) -> Result<PeersMember, crate::Failure> {
        PeersMember::checked(
            Some(member(servers, relay_only)),
            &granted(),
            Some(&outgoing()),
        )
    }

    #[test]
    fn stun_and_turn_servers_on_declared_hosts_are_accepted() {
        for url in [
            "stun:stun.example.com:3478",
            "turn:turn.example.com:3478",
            "turn:turn.example.com:3478?transport=udp",
            "turn:turn.example.com:3478?transport=tcp",
            "turns:turn.example.com:5349?transport=tcp",
            "turns:turn.example.com:5349",
            "turn:127.0.0.1:3478",
            "turn:localhost:3478",
            "turn:[::1]:3478",
        ] {
            assert!(checked(&[url], false).is_ok(), "{url}");
        }
    }

    #[test]
    fn a_server_that_is_not_a_plain_stun_or_turn_url_is_refused() {
        for url in [
            "",
            "https://turn.example.com",
            "turn:turn.example.com",
            "turn:turn.example.com:0",
            "turn:turn.example.com:70000",
            "turn:user@turn.example.com:3478",
            "turn:turn.example.com:3478?transport=sctp",
            "turns:turn.example.com:5349?transport=udp",
            "stun:stun.example.com:3478?transport=udp",
            "turn:turn.example.com/path:3478",
            "turn: turn.example.com:3478",
        ] {
            assert!(checked(&[url], false).is_err(), "{url}");
        }
        let long = format!("turn:{}.example.com:3478", "a".repeat(260));
        assert!(checked(&[&long], false).is_err());
    }

    #[test]
    fn a_server_whose_host_is_not_declared_is_refused_with_the_host_named() {
        let failure = checked(&["turn:relay.other.org:3478"], false).expect_err("undeclared");
        assert!(failure.to_string().contains("relay.other.org"), "{failure}");
    }

    #[test]
    fn relay_only_needs_a_turn_server_and_the_list_is_bounded() {
        assert!(checked(&["stun:stun.example.com:3478"], true).is_err());
        assert!(checked(&["turn:turn.example.com:3478"], true).is_ok());
        let many = vec!["stun:stun.example.com:3478"; 9];
        assert!(checked(&many, false).is_err());
        assert!(checked(&[], false).is_ok());
    }

    #[test]
    fn the_member_needs_the_grant_and_the_grant_alone_has_no_servers() {
        let refused = PeersMember::checked(Some(member(&[], false)), &[], None);
        assert!(refused.is_err());
        let alone = PeersMember::checked(None, &granted(), None).expect("grant alone");
        assert_eq!(alone, PeersMember::default());
        assert_eq!(
            alone.meta_tag(),
            "<meta name=\"tachyon-peers\" content=\"{&quot;iceServers&quot;:[],&quot;relayOnly&quot;:false}\">"
        );
    }
}
