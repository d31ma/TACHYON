use crate::native::SiteSettings;

pub(super) struct GrantedScript {
    pub(super) grant: &'static str,
    pub(super) script_tag: &'static str,
    pub(super) insecure_note: &'static str,
    pub(super) source: &'static str,
}

impl GrantedScript {
    const NO_TAG: &'static str = "";
    const SECURE_STARTS: [&'static str; 3] = ["https://", "http://localhost", "http://127.0.0.1"];

    fn granted(&self, site: &SiteSettings) -> bool {
        site.browser_apis.iter().any(|grant| grant == self.grant)
    }

    pub(super) fn tag(&self, site: &SiteSettings) -> &'static str {
        [Self::NO_TAG, self.script_tag][usize::from(self.granted(site))]
    }

    pub(super) fn note(&self, site: &SiteSettings) -> Option<String> {
        let wanted = [
            self.granted(site),
            site.origin.as_deref().is_some_and(Self::insecure),
        ] == [true, true];
        wanted.then(|| String::from(self.insecure_note))
    }

    fn insecure(origin: &str) -> bool {
        !Self::SECURE_STARTS
            .iter()
            .any(|start| origin.starts_with(start))
    }

    pub(super) fn script(&self) -> Vec<u8> {
        crate::template::ShippedScript::compact(self.source).into_bytes()
    }
}
