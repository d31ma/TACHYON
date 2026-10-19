use super::browser_api_permissions::BrowserApiPermissions;

pub(super) struct LinuxBrowserApis;

impl LinuxBrowserApis {
    const SOURCE: &'static str = include_str!("linux_browser_apis.c");

    pub(super) fn source(permissions: &BrowserApiPermissions) -> String {
        Self::SOURCE.replace("__GRANTED_BROWSER_APIS__", &permissions.c_grant_checks())
    }
}
