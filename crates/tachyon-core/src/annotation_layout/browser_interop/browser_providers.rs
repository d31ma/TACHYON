use super::browser_provider::BrowserProvider;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct BrowserProviders;

impl BrowserProviders {
    const PREFERENCE: BrowserProvider = BrowserProvider::new(
        "PreferenceStrings",
        &["java", "js", "ts"],
        "PreferenceStrings currently has Java browser and JavaScript/TypeScript page providers. Keep raw-string browser preferences in those services and pass results through the existing typed event contract. A linked native or another language provider requires its own adapter.",
    );
    const TITLE: BrowserProvider = BrowserProvider::new(
        "DocumentTitle",
        &["kt", "js", "ts"],
        "DocumentTitle currently has Kotlin browser and JavaScript/TypeScript page providers. Inject it into an owned browser service. A linked native or another language provider requires its own adapter.",
    );

    pub(super) fn browser(path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut problems = Self::PREFERENCE.browser(path, source);
        problems.extend(Self::TITLE.browser(path, source));
        problems
    }

    pub(super) fn native(path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut problems = Self::PREFERENCE.native(path, source);
        problems.extend(Self::TITLE.native(path, source));
        problems
    }
}
