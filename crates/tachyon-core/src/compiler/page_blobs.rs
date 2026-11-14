use super::PageHead;
use super::page_files::PageFiles;
use crate::native::SiteSettings;

pub(super) struct PageBlobs;

impl PageBlobs {
    pub(super) const PATH: &'static str = ".tachyon/tac-blobs.js";
    const GRANT: &'static str = "blobs";
    const SOURCE: &'static str = include_str!("../template/tac-blobs.js");
    const SCRIPT_TAG: &'static str =
        r#"<script type="module" src="/.tachyon/tac-blobs.js" data-tachyon-runtime></script>"#;

    pub(super) fn tag(site: &SiteSettings) -> String {
        Self::tags_when(Self::granted(site), site)
    }

    fn tags_when(granted: bool, site: &SiteSettings) -> String {
        if !granted {
            return String::new();
        }
        Self::granted_tags(site)
    }

    fn granted(site: &SiteSettings) -> bool {
        site.browser_apis.iter().any(|grant| grant == Self::GRANT)
    }

    fn granted_tags(site: &SiteSettings) -> String {
        let limits = site.blob_limits.content();
        format!(
            "<meta name=\"tachyon-blobs\" content=\"{limits}\">{}{}",
            Self::hosts_tag(site),
            Self::SCRIPT_TAG
        )
    }

    fn hosts_tag(site: &SiteSettings) -> String {
        let hosts = PageHead::escape(&site.outgoing_hosts.join(" "));
        let tags = [
            format!("<meta name=\"tachyon-outgoing-hosts\" content=\"{hosts}\">"),
            String::new(),
        ];
        tags[usize::from(PageFiles::wanted(site))].clone()
    }

    pub(super) fn script() -> Vec<u8> {
        super::page_transfer::PageTransfer::script_with(Self::SOURCE)
    }
}
