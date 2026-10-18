#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AppOrigins {
    origins: Vec<String>,
}

impl AppOrigins {
    const SEPARATOR: &'static str = "://";
    const MAX_ORIGIN_LENGTH: usize = 256;

    pub(crate) fn parse(entries: &[&str]) -> Option<Self> {
        let origins = entries
            .iter()
            .map(|entry| Self::written_as_origin(entry))
            .collect::<Option<Vec<_>>>()?;
        Some(Self { origins })
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.origins.is_empty()
    }

    pub(crate) fn allows(&self, origin: &str) -> bool {
        Self::origin(origin).is_some_and(|origin| self.origins.contains(&origin))
    }

    fn written_as_origin(entry: &str) -> Option<String> {
        Self::origin(entry).filter(|origin| origin == entry)
    }

    fn origin(text: &str) -> Option<String> {
        let address = url::Url::parse(text)
            .ok()
            .filter(|_| text.len() <= Self::MAX_ORIGIN_LENGTH)?;
        let host = address.host_str().filter(|_| Self::is_bare(&address))?;
        let port = address.port().map(|port| format!(":{port}"));
        Some(format!(
            "{}{}{host}{}",
            address.scheme(),
            Self::SEPARATOR,
            port.unwrap_or_default()
        ))
    }

    fn is_bare(address: &url::Url) -> bool {
        let bare = [
            address.username().is_empty(),
            address.password().is_none(),
            address.query().is_none(),
            address.fragment().is_none(),
            matches!(address.path(), "" | "/"),
        ];
        bare == [true; 5]
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::AppOrigins;

    #[test]
    fn the_fixed_native_app_origins_and_web_origins_are_accepted() {
        let listed = [
            "tachyon-app://bundle",
            "https://appassets.tachyon.local",
            "https://tachyon.local",
            "tachyon://app",
            "https://app.example.com",
            "http://127.0.0.1:4173",
        ];
        let origins = AppOrigins::parse(&listed).expect("origins");
        for origin in listed {
            assert!(origins.allows(origin), "{origin}");
        }
        assert!(origins.allows("https://APP.example.com"));
        assert!(origins.allows("https://app.example.com:443"));
        assert!(!origins.is_empty());
    }

    #[test]
    fn origins_not_listed_or_not_origins_are_refused() {
        let origins = AppOrigins::parse(&["https://app.example.com"]).expect("origins");
        for origin in [
            "null",
            "",
            "https://evil.example.com",
            "https://app.example.com.evil.com",
            "http://app.example.com",
            "https://app.example.com:8443",
        ] {
            assert!(!origins.allows(origin), "{origin}");
        }
        assert!(!AppOrigins::default().allows("https://app.example.com"));
        assert!(AppOrigins::default().is_empty());
    }

    #[test]
    fn a_setting_must_be_written_exactly_as_an_origin() {
        for entry in [
            "app.example.com",
            "https://app.example.com/",
            "https://app.example.com/path",
            "https://user@app.example.com",
            "https://app.example.com?x=1",
            "https://APP.example.com",
            "https://app.example.com:443",
            "tachyon-app://bundle/",
            "*",
            "null",
        ] {
            assert_eq!(AppOrigins::parse(&[entry]), None, "{entry}");
        }
    }
}
