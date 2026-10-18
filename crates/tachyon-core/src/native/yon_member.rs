use super::config::config_failure;
use crate::Failure;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct YonMember {
    origin: String,
}

impl YonMember {
    const MAX_ORIGIN_LENGTH: usize = 256;
    const META_NAME: &'static str = "tachyon-yon-origin";
    const SECURE_SCHEME: &'static str = "https";
    const LOCAL_SCHEME: &'static str = "http";
    const ORIGIN_MESSAGE: &'static str = "Manifest tac.yon.origin must be the https origin of the Yon server, with no path and no trailing slash, such as https://api.example.com. A local Yon may use http://localhost:8080 or http://127.0.0.1:8080.";

    pub(crate) fn validate(&self) -> Result<(), Failure> {
        Self::is_origin(&self.origin)
            .then_some(())
            .ok_or_else(|| config_failure(Self::ORIGIN_MESSAGE))
    }

    pub(crate) fn into_origin(self) -> String {
        self.origin
    }

    pub(crate) fn meta_tag(origin: &str) -> String {
        format!(
            "<meta name=\"{}\" content=\"{}\">",
            Self::META_NAME,
            crate::compiler::html_attribute_escape(origin)
        )
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
            origin: String::from(origin),
        }
        .validate()
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
}
