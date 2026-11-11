use crate::native::SiteSettings;

pub(super) struct WebPush;

impl WebPush {
    const GRANT: &'static str = "push";
    const PAGE_PATH: &'static str = ".tachyon/tac-web-push.js";
    const WORKER_PATH: &'static str = ".tachyon/tac-push-sw.js";
    const PAGE_SOURCE: &'static str = include_str!("../template/tac-web-push.js");
    const WORKER_SOURCE: &'static str = include_str!("../template/tac-push-sw.js");
    const NO_TAG: &'static str = "";
    const SCRIPT_TAG: &'static str =
        r#"<script type="module" src="/.tachyon/tac-web-push.js" data-tachyon-runtime></script>"#;

    fn granted(site: &SiteSettings) -> bool {
        site.browser_apis.iter().any(|grant| grant == Self::GRANT)
    }

    pub(super) fn tag(site: &SiteSettings, native: bool) -> &'static str {
        let wanted = [Self::granted(site), native] == [true, false];
        [Self::NO_TAG, Self::SCRIPT_TAG][usize::from(wanted)]
    }

    pub(super) fn files() -> [(&'static str, Vec<u8>); 2] {
        [
            (Self::PAGE_PATH, Self::compact(Self::PAGE_SOURCE)),
            (Self::WORKER_PATH, Self::compact(Self::WORKER_SOURCE)),
        ]
    }

    fn compact(source: &str) -> Vec<u8> {
        crate::template::ShippedScript::compact(source).into_bytes()
    }
}
