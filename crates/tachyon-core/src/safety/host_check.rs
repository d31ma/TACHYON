use super::{CodeText, DeclaredHosts, OutgoingUrl, SafetyUtils};
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct HostCheck<'h> {
    tac: &'h DeclaredHosts,
    yon: &'h DeclaredHosts,
}

impl<'h> HostCheck<'h> {
    const CODE: u16 = 1326;
    const CLIENT: &'static str = "client";
    const BUILT_HOST: &'static str = "a host that the code builds while it runs";
    const HELP: &'static str = "List every host that the code sends requests to, as a host name without scheme or port (\"api.example.com\", or \"*.example.com\" for its subdomains): tac.outgoing.hosts in client/manifest.json for page code, outgoing.hosts in server/application.json for Yon code. Tachyon refuses requests to other hosts when it builds and when it runs, so a caller cannot make your app call an address that they choose (server-side request forgery, SSRF). A host that the code builds while it runs cannot be checked: start the URL with a declared host (ADR 0068).";

    pub(crate) const fn new(tac: &'h DeclaredHosts, yon: &'h DeclaredHosts) -> Self {
        Self { tac, yon }
    }

    pub(crate) fn diagnostics<'a>(
        &self,
        files: impl Iterator<Item = (&'a Path, &'a [u8])>,
    ) -> Vec<Diagnostic> {
        SafetyUtils::each_file(files, |path, bytes| self.file_diagnostics(path, bytes))
    }

    fn file_diagnostics(&self, path: &Path, bytes: &[u8]) -> Vec<Diagnostic> {
        let text = CodeText::source(path, bytes);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let code = CodeText::of(extension, text);
        let hosts = self.hosts_for(path);
        OutgoingUrl::find(text, &code)
            .into_iter()
            .filter(|url| !hosts.allows(&url.host))
            .map(|url| Self::diagnostic(path, text, &url, hosts))
            .collect()
    }

    fn hosts_for(&self, path: &Path) -> &'h DeclaredHosts {
        if path.starts_with(Self::CLIENT) {
            return self.tac;
        }
        self.yon
    }

    fn diagnostic(path: &Path, text: &str, url: &OutgoingUrl, hosts: &DeclaredHosts) -> Diagnostic {
        let portable = SafetyUtils::portable(path);
        let message = format!(
            "'{portable}' line {} sends a request to {}, but {} does not list it.",
            SafetyUtils::line_at(text, url.at),
            Self::host_words(&url.host),
            hosts.setting()
        );
        SafetyUtils::refusal(Self::CODE, message, Self::HELP, (&portable, url.at))
    }

    fn host_words(host: &str) -> String {
        if host.is_empty() {
            return String::from(Self::BUILT_HOST);
        }
        format!("'{host}'")
    }
}
