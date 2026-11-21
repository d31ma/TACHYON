use crate::native::SiteSettings;

pub(super) struct PageRecorder;

impl PageRecorder {
    pub(super) const PATH: &'static str = ".tachyon/tac-recorder.js";
    const SOURCE: &'static str = include_str!("../template/tac-recorder.js");
    const FILES_GRANT: &'static str = "files";
    const MEDIA_GRANTS: [&'static str; 3] = ["camera", "microphone", "screen"];
    const NO_TAG: &'static str = "";
    const SCRIPT_TAG: &'static str =
        r#"<script type="module" src="/.tachyon/tac-recorder.js" data-tachyon-runtime></script>"#;

    pub(super) fn granted(site: &SiteSettings) -> bool {
        let grants = &site.browser_apis;
        let files = grants.iter().any(|grant| grant == Self::FILES_GRANT);
        let media = grants
            .iter()
            .any(|grant| Self::MEDIA_GRANTS.contains(&grant.as_str()));
        [files, media] == [true, true]
    }

    pub(super) fn tag(site: &SiteSettings) -> &'static str {
        [Self::NO_TAG, Self::SCRIPT_TAG][usize::from(Self::granted(site))]
    }

    pub(super) fn script() -> Vec<u8> {
        crate::template::ShippedScript::compact(Self::SOURCE).into_bytes()
    }
}
