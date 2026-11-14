use super::page_module::PageModule;
use crate::native::SiteSettings;

pub(super) struct PagePeers;

impl PagePeers {
    pub(super) const PATH: &'static str = PageModule::PEERS.path;

    pub(super) fn tag(site: &SiteSettings) -> String {
        Self::with_settings(PageModule::PEERS.tag(site), site)
    }

    fn with_settings(script_tag: &str, site: &SiteSettings) -> String {
        if script_tag.is_empty() {
            return String::new();
        }
        format!("{}{script_tag}", site.peers.meta_tag())
    }

    pub(super) fn script() -> Vec<u8> {
        PageModule::PEERS.script()
    }
}
