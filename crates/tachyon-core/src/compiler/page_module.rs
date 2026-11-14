use crate::native::SiteSettings;

pub(super) struct PageModule {
    pub(super) path: &'static str,
    grant: &'static str,
    source: &'static str,
    script_tag: &'static str,
    insecure_note: &'static str,
}

impl PageModule {
    pub(super) const PASSKEYS: Self = Self {
        path: ".tachyon/tac-passkeys.js",
        grant: "passkeys",
        source: include_str!("../template/tac-passkeys.js"),
        script_tag: r#"<script type="module" src="/.tachyon/tac-passkeys.js" data-tachyon-runtime></script>"#,
        insecure_note: "Passkeys need a secure page: tac.origin is not https, so WebAuthn works there only on localhost during development (ADR 0126).",
    };
    pub(super) const CRYPTO: Self = Self {
        path: ".tachyon/tac-crypto.js",
        grant: "crypto",
        source: include_str!("../template/tac-crypto.js"),
        script_tag: r#"<script type="module" src="/.tachyon/tac-crypto.js" data-tachyon-runtime></script>"#,
        insecure_note: "Crypto needs a secure page: tac.origin is not https, so WebCrypto works there only on localhost during development, and every other Crypto call answers CryptoFailed with reason unsupported (ADR 0132).",
    };
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
