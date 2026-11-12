use crate::native::SiteSettings;

pub(super) struct PagePasskeys;

impl PagePasskeys {
    pub(super) const PATH: &'static str = ".tachyon/tac-passkeys.js";
    const GRANT: &'static str = "passkeys";
    const SOURCE: &'static str = include_str!("../template/tac-passkeys.js");
    const NO_TAG: &'static str = "";
    const SECURE_STARTS: [&'static str; 3] = ["https://", "http://localhost", "http://127.0.0.1"];
    const INSECURE_NOTE: &'static str = "Passkeys need a secure page: tac.origin is not https, so WebAuthn works there only on localhost during development (ADR 0126).";
    const SCRIPT_TAG: &'static str =
        r#"<script type="module" src="/.tachyon/tac-passkeys.js" data-tachyon-runtime></script>"#;

    fn granted(site: &SiteSettings) -> bool {
        site.browser_apis.iter().any(|grant| grant == Self::GRANT)
    }

    pub(super) fn tag(site: &SiteSettings) -> &'static str {
        [Self::NO_TAG, Self::SCRIPT_TAG][usize::from(Self::granted(site))]
    }

    pub(super) fn note(site: &SiteSettings) -> Option<String> {
        let wanted = [
            Self::granted(site),
            site.origin.as_deref().is_some_and(Self::insecure),
        ] == [true, true];
        wanted.then(|| String::from(Self::INSECURE_NOTE))
    }

    fn insecure(origin: &str) -> bool {
        !Self::SECURE_STARTS
            .iter()
            .any(|start| origin.starts_with(start))
    }

    pub(super) fn script() -> Vec<u8> {
        crate::template::ShippedScript::compact(Self::SOURCE).into_bytes()
    }
}
