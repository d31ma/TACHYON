use super::browser_api_permissions::BrowserApiPermissions;

pub(super) struct LinuxCrypto;

impl LinuxCrypto {
    pub(super) const SOURCE: &'static str = include_str!("linux_crypto.c");
    const REFUSED: &'static str = include_str!("linux_crypto_refused.c");
    const GRANT: &'static str = "crypto";
    const PKG_CONFIG_MODULES: [&'static str; 2] = ["libsecret-1", "libcrypto"];
    const NO_MODULES: [&'static str; 0] = [];
    const PACKAGE_DEPENDS: &'static str = ", libsecret-1-0, libssl3t64 | libssl3";

    pub(super) fn source(permissions: &BrowserApiPermissions) -> &'static str {
        [Self::REFUSED, Self::SOURCE][usize::from(Self::granted(permissions))]
    }

    pub(super) fn pkg_config_modules(
        permissions: &BrowserApiPermissions,
    ) -> &'static [&'static str] {
        [&Self::NO_MODULES[..], &Self::PKG_CONFIG_MODULES[..]]
            [usize::from(Self::granted(permissions))]
    }

    pub(super) fn package_depends(permissions: &BrowserApiPermissions) -> &'static str {
        ["", Self::PACKAGE_DEPENDS][usize::from(Self::granted(permissions))]
    }

    fn granted(permissions: &BrowserApiPermissions) -> bool {
        permissions.is_granted(Self::GRANT)
    }
}
