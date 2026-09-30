use super::SpeculationRules;

pub(crate) struct ContentSecurityPolicy;

impl ContentSecurityPolicy {
    const SCRIPT_SOURCES: &'static str = "'self' 'wasm-unsafe-eval'";

    pub(crate) fn server(connect_sources: &str) -> String {
        format!(
            "default-src 'self'; connect-src 'self' {connect_sources}; style-src 'self' 'unsafe-inline'; img-src 'self' data:; \
             script-src {}; object-src 'none'; \
             base-uri 'none'; frame-ancestors 'none'",
            Self::script_sources()
        )
    }

    pub(crate) fn native_host() -> String {
        format!(
            "default-src 'self'; script-src {}; style-src 'self' 'unsafe-inline'; \
             img-src 'self' data: https:; font-src 'self' data: https:; connect-src 'self' https:; \
             frame-src 'none'; object-src 'none'; base-uri 'self'",
            Self::script_sources()
        )
    }

    fn script_sources() -> String {
        format!(
            "{} {}",
            Self::SCRIPT_SOURCES,
            SpeculationRules::script_source()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ContentSecurityPolicy;
    use crate::compiler::SpeculationRules;

    #[test]
    fn the_server_and_the_native_hosts_allow_the_same_scripts() {
        let scripts = format!(
            "script-src 'self' 'wasm-unsafe-eval' {};",
            SpeculationRules::script_source()
        );
        let server = ContentSecurityPolicy::server("https://api.example.com");
        let native = ContentSecurityPolicy::native_host();
        assert!(server.contains(&scripts), "{server}");
        assert!(native.contains(&scripts), "{native}");
        assert!(
            server.contains("connect-src 'self' https://api.example.com;"),
            "{server}"
        );
        assert!(!native.contains('"'), "{native}");
    }
}
