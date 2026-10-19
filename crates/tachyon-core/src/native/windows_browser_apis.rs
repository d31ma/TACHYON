use super::browser_api_permissions::BrowserApiPermissions;

pub(super) struct WindowsBrowserApis;

impl WindowsBrowserApis {
    pub(super) const SOURCE: &'static str = include_str!("windows_browser_apis.c");

    pub(super) fn source(permissions: &BrowserApiPermissions) -> String {
        Self::SOURCE.replace("__GRANTED_BROWSER_APIS__", &permissions.c_grant_checks())
    }
}
