use super::{SpeculationRules, ViewTransitionGuard};
use sha2::{Digest, Sha256};

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

    pub(crate) fn inline_script_hash(script: &str) -> String {
        let digest = Sha256::digest(script.as_bytes());
        format!("'sha256-{}'", crate::tac_web::base64(&digest))
    }

    fn script_sources() -> String {
        format!(
            "{} {} {}",
            Self::SCRIPT_SOURCES,
            SpeculationRules::script_source(),
            ViewTransitionGuard::script_source()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ContentSecurityPolicy;
    use crate::compiler::{SpeculationRules, ViewTransitionGuard};

    #[test]
    fn the_server_and_the_native_hosts_allow_the_same_scripts() {
        let scripts = format!(
            "script-src 'self' 'wasm-unsafe-eval' {} {};",
            SpeculationRules::script_source(),
            ViewTransitionGuard::script_source()
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
