use super::{SpeculationRules, ViewTransitionGuard};
use sha2::{Digest, Sha256};

pub(crate) struct ContentSecurityPolicy;

impl ContentSecurityPolicy {
    const SCRIPT_SOURCES: &'static str = "'self' 'wasm-unsafe-eval'";
    const SELF_SOURCE: &'static str = "'self'";
    const NATIVE_BOOTSTRAP: &'static str = "default-src 'none'; script-src 'unsafe-inline'; \
         base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

    pub(crate) fn server(connect_sources: &str) -> String {
        format!(
            "default-src 'self'; connect-src 'self' {connect_sources}; style-src 'self' 'unsafe-inline'; img-src 'self' data:; \
             script-src {}; object-src 'none'; \
             base-uri 'none'; frame-ancestors 'none'",
            Self::script_sources()
        )
    }

    pub(crate) fn native_host(connect_sources: &str) -> String {
        Self::native_host_allowing(connect_sources, "")
    }

    pub(crate) fn native_host_with_inline_script(connect_sources: &str, script: &str) -> String {
        let script_hash = format!(" {}", Self::inline_script_hash(script));
        Self::native_host_allowing(connect_sources, &script_hash)
    }

    pub(crate) fn native_bootstrap() -> &'static str {
        Self::NATIVE_BOOTSTRAP
    }

    fn native_host_allowing(connect_sources: &str, extra_scripts: &str) -> String {
        format!(
            "default-src 'self'; script-src {}{extra_scripts}; style-src 'self' 'unsafe-inline'; \
             img-src 'self' data: https:; font-src 'self' data: https:; connect-src {}; \
             frame-src 'none'; object-src 'none'; base-uri 'self'",
            Self::script_sources(),
            Self::connect_list(connect_sources)
        )
    }

    fn connect_list(connect_sources: &str) -> String {
        [Self::SELF_SOURCE, connect_sources]
            .into_iter()
            .filter(|source| !source.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
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
        let native = ContentSecurityPolicy::native_host("");
        assert!(server.contains(&scripts), "{server}");
        assert!(native.contains(&scripts), "{native}");
        assert!(
            server.contains("connect-src 'self' https://api.example.com;"),
            "{server}"
        );
        assert!(!native.contains('"'), "{native}");
    }

    #[test]
    fn the_native_hosts_connect_only_to_the_declared_hosts() {
        let closed = ContentSecurityPolicy::native_host("");
        assert!(closed.contains("connect-src 'self';"), "{closed}");
        assert!(!closed.contains("connect-src 'self' https:;"), "{closed}");
        let listed = ContentSecurityPolicy::native_host("https://api.example.com");
        assert!(
            listed.contains("connect-src 'self' https://api.example.com;"),
            "{listed}"
        );
    }

    #[test]
    fn a_native_host_that_writes_a_script_into_documents_allows_only_that_script() {
        let script = "globalThis.ready = true;";
        let policy = ContentSecurityPolicy::native_host_with_inline_script("", script);
        let hash = ContentSecurityPolicy::inline_script_hash(script);
        assert!(policy.contains(&format!(" {hash}; style-src")), "{policy}");
        let scripts = policy
            .split(';')
            .find(|rule| rule.trim_start().starts_with("script-src"))
            .unwrap_or_default();
        assert!(!scripts.contains("'unsafe-inline'"), "{policy}");
        assert!(!ContentSecurityPolicy::native_bootstrap().contains('"'));
    }
}
