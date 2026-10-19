use super::native_links::NativeLinks;

pub(super) struct NativeDeepLink;

impl NativeDeepLink {
    const C_HELPER: &'static str = include_str!("native_deep_link.h");
    const SWIFT_HELPER: &'static str = include_str!("TacDeepLink.swift");
    const JAVA_HELPER: &'static str = include_str!("native_deep_link_android.java");
    pub(super) const WINDOWS: &'static str = include_str!("windows_deep_links.c");
    const SCHEMES: &'static str = "__LINK_SCHEMES__";
    const HOSTS: &'static str = "__LINK_HOSTS__";

    pub(super) fn c_helper(links: &NativeLinks) -> String {
        let schemes = Self::c_entries(&links.quoted_schemes("", ""));
        let hosts = Self::c_entries(&links.quoted_hosts("", ""));
        Self::C_HELPER
            .replace(Self::SCHEMES, &schemes)
            .replace(Self::HOSTS, &hosts)
    }

    pub(super) fn swift_helper(links: &NativeLinks) -> String {
        Self::SWIFT_HELPER
            .replace(Self::SCHEMES, &links.quoted_schemes("[", "]"))
            .replace(Self::HOSTS, &links.quoted_hosts("[", "]"))
    }

    pub(super) fn java_helper(links: &NativeLinks) -> String {
        Self::JAVA_HELPER
            .replace(Self::SCHEMES, &links.quoted_schemes("{", "}"))
            .replace(Self::HOSTS, &links.quoted_hosts("{", "}"))
    }

    fn c_entries(quoted: &str) -> String {
        quoted
            .split(", ")
            .filter(|entry| !entry.is_empty())
            .map(|entry| format!("{entry}, "))
            .collect::<Vec<_>>()
            .concat()
    }
}
