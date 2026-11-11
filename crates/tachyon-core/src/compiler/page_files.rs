use super::PageHead;
use crate::native::SiteSettings;

pub(super) struct PageFiles;

impl PageFiles {
    pub(super) const PATH: &'static str = ".tachyon/tac-files.js";
    const GRANT: &'static str = "files";
    const SOURCE: &'static str = concat!(
        include_str!("../template/tac-sha256.js"),
        "\n",
        include_str!("../template/tac-files.js")
    );
    const SCRIPT_TAG: &'static str =
        r#"<script type="module" src="/.tachyon/tac-files.js" data-tachyon-runtime></script>"#;

    pub(super) fn wanted(site: &SiteSettings) -> bool {
        site.browser_apis.iter().any(|grant| grant == Self::GRANT)
    }

    pub(super) fn script() -> Vec<u8> {
        crate::template::ShippedScript::compact(Self::SOURCE).into_bytes()
    }

    pub(super) fn tag(site: &SiteSettings) -> String {
        let tags = [String::new(), Self::granted_tags(site)];
        tags[usize::from(Self::wanted(site))].clone()
    }

    fn granted_tags(site: &SiteSettings) -> String {
        let hosts = PageHead::escape(&site.outgoing_hosts.join(" "));
        format!(
            "<meta name=\"tachyon-outgoing-hosts\" content=\"{hosts}\">{}",
            Self::SCRIPT_TAG
        )
    }
}
