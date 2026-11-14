use crate::native::SiteSettings;

pub(super) struct PageMedia;

impl PageMedia {
    const GRANTS: [&'static str; 3] = ["camera", "microphone", "screen"];
    const SCRIPTS: [(&'static str, &'static str); 2] = [
        (
            ".tachyon/tac-media.js",
            include_str!("../template/tac-media.js"),
        ),
        (
            ".tachyon/tac-media-view.js",
            include_str!("../template/tac-media-view.js"),
        ),
    ];
    const NO_TAG: &'static str = "";
    const SCRIPT_TAGS: &'static str = concat!(
        r#"<script type="module" src="/.tachyon/tac-media.js" data-tachyon-runtime></script>"#,
        r#"<script type="module" src="/.tachyon/tac-media-view.js" data-tachyon-runtime></script>"#
    );

    pub(super) fn granted(site: &SiteSettings) -> bool {
        site.browser_apis
            .iter()
            .any(|grant| Self::GRANTS.contains(&grant.as_str()))
    }

    pub(super) fn tag(site: &SiteSettings) -> &'static str {
        [Self::NO_TAG, Self::SCRIPT_TAGS][usize::from(Self::granted(site))]
    }

    pub(super) fn files() -> [(&'static str, Vec<u8>); 2] {
        Self::SCRIPTS.map(|(path, source)| (path, Self::shipped(source)))
    }

    fn shipped(source: &str) -> Vec<u8> {
        crate::template::ShippedScript::compact(source).into_bytes()
    }
}
