use super::config::config_failure;
use crate::Failure;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct YonMember {
    #[serde(default)]
    origin: Option<String>,
    #[serde(default)]
    origins: BTreeMap<String, String>,
}

impl YonMember {
    const MAX_ORIGIN_LENGTH: usize = 256;
    const MOST_NAMED_ORIGINS: usize = 8;
    const META_NAME: &'static str = "tachyon-yon-origin";
    const ORIGINS_META_NAME: &'static str = "tachyon-yon-origins";
    const HOSTS_POINTER: &'static str = "/hosts";
    const HOSTS_SETTING: &'static str = "tac.outgoing.hosts in client/manifest.json";
    const SECURE_SCHEME: &'static str = "https";
    const LOCAL_SCHEME: &'static str = "http";
    const ORIGIN_MESSAGE: &'static str = "Manifest tac.yon.origin must be the https origin of the Yon server, with no path and no trailing slash, such as https://api.example.com. A local Yon may use http://localhost:8080 or http://127.0.0.1:8080.";
    const NAMED_ORIGIN_MESSAGE: &'static str = "Each value in manifest tac.yon.origins must be an https origin with no path and no trailing slash, such as {\"realtime\": \"https://rt.example.com\"}. A local server may use http://localhost:8080 or http://127.0.0.1:8080.";
    const NAME_MESSAGE: &'static str = "Each name in manifest tac.yon.origins starts with a lowercase letter and has at most 32 lowercase letters, digits and dashes, such as \"realtime\".";
    const COUNT_MESSAGE: &'static str = "Manifest tac.yon.origins names at most 8 origins.";

    pub(crate) fn validate(&self, outgoing: Option<&serde_json::Value>) -> Result<(), Failure> {
        self.origin
            .iter()
            .try_for_each(|origin| Self::check_origin(origin))?;
        Self::check_count(self.origins.len())?;
        let hosts = Self::declared_hosts(outgoing);
        self.origins
            .iter()
            .try_for_each(|(name, origin)| Self::check_named(name, origin, &hosts))
    }

    pub(crate) fn origin(&self) -> Option<String> {
        self.origin.clone()
    }

    pub(crate) fn origins(&self) -> BTreeMap<String, String> {
        self.origins.clone()
    }

    pub(crate) fn meta_tag(origin: &str) -> String {
        format!(
            "<meta name=\"{}\" content=\"{}\">",
            Self::META_NAME,
            crate::compiler::html_attribute_escape(origin)
        )
    }

    pub(crate) fn origins_meta_tag(origins: &BTreeMap<String, String>) -> String {
        let written = serde_json::to_string(origins).unwrap_or_default();
        format!(
            "<meta name=\"{}\" content=\"{}\">",
            Self::ORIGINS_META_NAME,
            crate::compiler::html_attribute_escape(&written)
        )
    }

    pub(crate) fn is_local_origin(origin: &str) -> bool {
        url::Url::parse(origin)
            .ok()
            .is_some_and(|address| Self::has_local_host(&address))
    }

    fn check_origin(origin: &str) -> Result<(), Failure> {
        Self::is_origin(origin)
            .then_some(())
            .ok_or_else(|| config_failure(Self::ORIGIN_MESSAGE))
    }

    fn check_count(count: usize) -> Result<(), Failure> {
        (count <= Self::MOST_NAMED_ORIGINS)
            .then_some(())
            .ok_or_else(|| config_failure(Self::COUNT_MESSAGE))
    }

    fn check_named(
        name: &str,
        origin: &str,
        hosts: &crate::safety::DeclaredHosts,
    ) -> Result<(), Failure> {
        Self::check_name(name)?;
        Self::is_origin(origin)
            .then_some(())
            .ok_or_else(|| config_failure(Self::NAMED_ORIGIN_MESSAGE))?;
        Self::check_declared_host(name, origin, hosts)
    }

    fn check_name(name: &str) -> Result<(), Failure> {
        crate::behavior::RemoteOrigin::is_name(name)
            .then_some(())
            .ok_or_else(|| config_failure(Self::NAME_MESSAGE))
    }

    fn check_declared_host(
        name: &str,
        origin: &str,
        hosts: &crate::safety::DeclaredHosts,
    ) -> Result<(), Failure> {
        let host = Self::host_of(origin);
        let allowed = [Self::is_local_origin(origin), hosts.allows(&host)].contains(&true);
        allowed.then_some(()).ok_or_else(|| {
            config_failure(&format!(
                "Manifest tac.yon.origins.{name} is {origin}, but its host {host} is not in tac.outgoing.hosts. A Tac page reaches only declared hosts: add \"{host}\" to tac.outgoing.hosts in client/manifest.json."
            ))
        })
    }

    fn host_of(origin: &str) -> String {
        url::Url::parse(origin)
            .ok()
            .and_then(|address| address.host_str().map(str::to_owned))
            .unwrap_or_default()
    }

    fn declared_hosts(outgoing: Option<&serde_json::Value>) -> crate::safety::DeclaredHosts {
        let hosts = outgoing
            .and_then(|member| {
                crate::safety::DeclaredHosts::stored_hosts(member, Self::HOSTS_POINTER)
            })
            .unwrap_or_default();
        crate::safety::DeclaredHosts::named(&hosts, Self::HOSTS_SETTING)
    }

    fn is_origin(text: &str) -> bool {
        url::Url::parse(text)
            .ok()
            .filter(|_| text.len() <= Self::MAX_ORIGIN_LENGTH)
            .is_some_and(|address| Self::is_allowed_origin(&address, text))
    }

    fn is_allowed_origin(address: &url::Url, text: &str) -> bool {
        let written_as_origin = address.origin().ascii_serialization() == text;
        let secure = address.scheme() == Self::SECURE_SCHEME;
        let local = [
            address.scheme() == Self::LOCAL_SCHEME,
            Self::has_local_host(address),
        ];
        let allowed = [secure, local == [true, true]].contains(&true);
        [written_as_origin, allowed] == [true, true]
    }

    fn has_local_host(address: &url::Url) -> bool {
        address.host().is_some_and(|host| Self::is_local(&host))
    }

    fn is_local(host: &url::Host<&str>) -> bool {
        match host {
            url::Host::Domain(name) => crate::safety::LocalHostName::is_local(name),
            url::Host::Ipv4(address) => address.is_loopback(),
            url::Host::Ipv6(address) => address.is_loopback(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::YonMember;

    fn checked(origin: &str) -> Result<(), crate::Failure> {
        YonMember {
            origin: Some(String::from(origin)),
            origins: std::collections::BTreeMap::new(),
        }
        .validate(None)
    }

    #[test]
    fn an_https_origin_or_a_local_http_origin_is_accepted() {
        for origin in [
            "https://api.example.com",
            "https://api.example.com:8443",
            "http://localhost:8080",
            "http://app.localhost:3000",
            "http://127.0.0.1:8080",
            "http://[::1]:8080",
        ] {
            assert!(checked(origin).is_ok(), "{origin}");
        }
    }

    #[test]
    fn anything_that_is_not_exactly_an_origin_is_refused() {
        for origin in [
            "",
            "api.example.com",
            "https://api.example.com/",
            "https://api.example.com/api",
            "https://api.example.com?x=1",
            "https://user@api.example.com",
            "https://API.example.com",
            "https://api.example.com:443",
            "http://api.example.com",
            "http://10.0.0.5:8080",
            "ws://localhost:8080",
            "tachyon-app://bundle",
        ] {
            let text = checked(origin).expect_err(origin).to_string();
            assert!(text.contains("TY1601"), "{text}");
            assert!(text.contains("tac.yon.origin"), "{text}");
        }
    }

    #[test]
    fn the_page_learns_the_origin_from_one_meta_tag() {
        assert_eq!(
            YonMember::meta_tag("https://api.example.com"),
            "<meta name=\"tachyon-yon-origin\" content=\"https://api.example.com\">"
        );
    }

    fn named(origins: &str, hosts: &str) -> Result<(), crate::Failure> {
        let yon: YonMember =
            serde_json::from_str(&format!("{{\"origins\": {origins}}}")).expect("yon member");
        let outgoing: serde_json::Value =
            serde_json::from_str(&format!("{{\"hosts\": {hosts}}}")).expect("outgoing");
        yon.validate(Some(&outgoing))
    }

    #[test]
    fn a_named_origin_on_a_declared_host_or_a_local_server_is_accepted() {
        for (origins, hosts) in [
            (
                r#"{"realtime": "https://rt.example.com"}"#,
                r#"["rt.example.com"]"#,
            ),
            (
                r#"{"realtime": "https://rt.example.com:8443"}"#,
                r#"["*.example.com"]"#,
            ),
            (r#"{"local-2": "http://127.0.0.1:9000"}"#, "[]"),
        ] {
            assert!(named(origins, hosts).is_ok(), "{origins}");
        }
    }

    #[test]
    fn a_named_origin_must_name_a_declared_host_with_a_plain_name() {
        for (origins, hosts, expected) in [
            (
                r#"{"realtime": "https://rt.example.com"}"#,
                "[]",
                "add \"rt.example.com\" to tac.outgoing.hosts",
            ),
            (
                r#"{"realtime": "https://rt.example.com"}"#,
                r#"["api.example.com"]"#,
                "is not in tac.outgoing.hosts",
            ),
            (
                r#"{"Realtime": "https://rt.example.com"}"#,
                r#"["rt.example.com"]"#,
                "starts with a lowercase letter",
            ),
            (
                r#"{"9live": "https://rt.example.com"}"#,
                r#"["rt.example.com"]"#,
                "starts with a lowercase letter",
            ),
            (
                r#"{"realtime": "https://rt.example.com/ws"}"#,
                r#"["rt.example.com"]"#,
                "must be an https origin",
            ),
            (
                r#"{"realtime": "http://rt.example.com"}"#,
                r#"["rt.example.com"]"#,
                "must be an https origin",
            ),
            (
                r#"{"a": "http://localhost:1", "b": "http://localhost:2", "c": "http://localhost:3", "d": "http://localhost:4", "e": "http://localhost:5", "f": "http://localhost:6", "g": "http://localhost:7", "h": "http://localhost:8", "i": "http://localhost:9"}"#,
                "[]",
                "at most 8 origins",
            ),
        ] {
            let text = named(origins, hosts).expect_err(origins).to_string();
            assert!(text.contains("TY1601"), "{text}");
            assert!(text.contains(expected), "{origins}: {text}");
        }
    }

    #[test]
    fn the_page_learns_named_origins_from_one_meta_tag_and_csp_allows_them() {
        let origins = std::collections::BTreeMap::from([(
            String::from("realtime"),
            String::from("https://rt.example.com:8443"),
        )]);
        assert_eq!(
            YonMember::origins_meta_tag(&origins),
            "<meta name=\"tachyon-yon-origins\" content=\"{&quot;realtime&quot;:&quot;https://rt.example.com:8443&quot;}\">"
        );
        let sources = crate::safety::DeclaredHosts::named(&[], "test")
            .with_named_origins(origins.into_values().collect())
            .connect_sources();
        assert_eq!(
            sources,
            "https://rt.example.com:8443 wss://rt.example.com:8443"
        );
    }

    #[test]
    fn only_a_loopback_origin_is_local() {
        assert!(YonMember::is_local_origin("http://localhost:9000"));
        assert!(YonMember::is_local_origin("http://127.0.0.1:9000"));
        assert!(!YonMember::is_local_origin("https://rt.example.com"));
    }
}
