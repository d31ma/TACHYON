use crate::native::SiteSettings;

pub(super) struct YonReads;

impl YonReads {
    pub(super) const PATH: &'static str = ".tachyon/tac-yon-reads.js";
    const SOURCE: &'static str = include_str!("../template/tac-yon-reads.js");
    const NO_TAG: &'static str = "";
    const RULES_MARK: &'static str = "__CACHE_RULES__";
    const SCRIPT_TAG: &'static str =
        r#"<script type="module" src="/.tachyon/tac-yon-reads.js" data-tachyon-runtime></script>"#;

    fn wanted(site: &SiteSettings) -> bool {
        [site.yon_origin.is_some(), !site.cache_rules.is_empty()] == [true, true]
    }

    pub(super) fn script(cache_rules: &str) -> Vec<u8> {
        crate::template::ShippedScript::compact(
            &Self::SOURCE.replace(Self::RULES_MARK, cache_rules),
        )
        .into_bytes()
    }

    pub(super) fn tag(site: &SiteSettings) -> &'static str {
        [Self::NO_TAG, Self::SCRIPT_TAG][usize::from(Self::wanted(site))]
    }
}
