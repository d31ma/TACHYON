use super::config::NativeApplication;
use super::host::c_string_escape;
use super::routes::NativeRouteIndex;

pub(super) struct DesktopHostSource {
    pub(super) template: String,
    pub(super) menus: &'static str,
    pub(super) mnemonic: char,
}

impl DesktopHostSource {
    pub(super) fn render(
        &self,
        application: &NativeApplication,
        index: &NativeRouteIndex,
        platform_values: &[(&str, &str)],
    ) -> String {
        let framed = self.frame(application);
        let menu_items =
            super::desktop_menu_items::DesktopMenuItems::c_table(&application.menus, self.mnemonic);
        let app_name = c_string_escape(&application.name);
        let bundle_id = c_string_escape(&application.application_id);
        let entry_route = c_string_escape(&index.entry_route);
        let mut values = vec![
            ("__APP_NAME__", app_name.as_str()),
            ("__BUNDLE_ID__", bundle_id.as_str()),
            ("__ENTRY_ROUTE__", entry_route.as_str()),
            ("__NATIVE_MENU_ITEMS__", menu_items.as_str()),
            (
                "__EXTERNAL_LINK_HELPER__",
                super::native_external_link::NativeExternalLink::C_HELPER,
            ),
        ];
        values.extend_from_slice(platform_values);
        super::placeholder_fill::PlaceholderFill::once(&framed, &values)
    }

    fn frame(&self, application: &NativeApplication) -> String {
        self.template
            .replace(
                "__NATIVE_SHIM__",
                &c_string_escape(&super::host::native_shim(&application.window)),
            )
            .replace(
                "__CONTENT_SECURITY_POLICY__",
                &c_string_escape(&application.content_security_policy()),
            )
            .replace("__NATIVE_MENUS__", self.menus)
    }
}
