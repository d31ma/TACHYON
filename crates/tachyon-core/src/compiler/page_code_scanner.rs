use crate::native::SiteSettings;

pub(super) struct PageCodeScanner;

impl PageCodeScanner {
    pub(super) const PATH: &'static str = ".tachyon/tac-code-scanner.js";
    pub(super) const DECODER_PATH: &'static str = ".tachyon/tac-qr-decoder.js";
    const GRANT: &'static str = "camera";
    const SOURCE: &'static str = include_str!("../template/tac-code-scanner.js");
    const DECODER: &'static str = include_str!("../template/vendor/jsQR.js");
    const DECODER_NOTICE: &'static str = "// jsQR 1.4.0, Copyright Cosmo Wolfe and contributors, https://github.com/cozmo/jsQR\n// Licensed under the Apache License, Version 2.0: https://www.apache.org/licenses/LICENSE-2.0\n";
    const NO_TAG: &'static str = "";
    const SCRIPT_TAG: &'static str = r#"<script type="module" src="/.tachyon/tac-code-scanner.js" data-tachyon-runtime></script>"#;
    const REASON_START: &'static str = r#"<meta name="tachyon-camera-reason" content=""#;
    const REASON_END: &'static str = r#"">"#;

    pub(super) fn granted(site: &SiteSettings) -> bool {
        site.browser_apis.iter().any(|grant| grant == Self::GRANT)
    }

    pub(super) fn tag(site: &SiteSettings) -> String {
        let script = [Self::NO_TAG, Self::SCRIPT_TAG][usize::from(Self::granted(site))];
        [script, &Self::reason_tag(site)].concat()
    }

    fn reason_tag(site: &SiteSettings) -> String {
        site.camera_reason
            .as_deref()
            .map_or_else(String::new, Self::reason_meta)
    }

    fn reason_meta(reason: &str) -> String {
        [
            Self::REASON_START,
            &super::page_head::PageHead::escape(reason),
            Self::REASON_END,
        ]
        .concat()
    }

    pub(super) fn script() -> Vec<u8> {
        crate::template::ShippedScript::compact(Self::SOURCE).into_bytes()
    }

    pub(super) fn decoder() -> Vec<u8> {
        [Self::DECODER_NOTICE, Self::DECODER].concat().into_bytes()
    }
}
