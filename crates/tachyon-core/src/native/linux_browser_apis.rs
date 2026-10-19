use super::browser_api_permissions::BrowserApiPermissions;

pub(super) struct LinuxBrowserApis;

impl LinuxBrowserApis {
    const SOURCE: &'static str = include_str!("linux_browser_apis.c");
    const HOST_CHECKED: [&'static str; 4] = ["clipboard", "geolocation", "notifications", "share"];

    pub(super) fn source(permissions: &BrowserApiPermissions) -> String {
        let checks: Vec<String> = Self::HOST_CHECKED
            .iter()
            .filter(|api| permissions.is_granted(api))
            .map(|api| format!("if (strcmp(api, \"{api}\") == 0) return TRUE;"))
            .collect();
        Self::SOURCE.replace("__GRANTED_BROWSER_APIS__", &checks.join("\n  "))
    }
}
