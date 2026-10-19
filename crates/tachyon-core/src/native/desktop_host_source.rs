use super::config::NativeApplication;
use super::host::c_string_escape;
use super::routes::NativeRouteIndex;

pub(super) struct DesktopHostSource {
    pub(super) menus: &'static str,
    pub(super) mnemonic: char,
    pub(super) target: tachyon_contracts::NativeTarget,
}

impl DesktopHostSource {
    pub(super) fn render(
        &self,
        template: &str,
        application: &NativeApplication,
        index: &NativeRouteIndex,
        platform_values: &[(&str, &str)],
    ) -> String {
        let framed = self.frame(template, application);
        let menu_items =
            super::desktop_menu_items::DesktopMenuItems::c_table(&application.menus, self.mnemonic);
        let app_name = c_string_escape(&application.name);
        let bundle_id = c_string_escape(&application.application_id);
        let entry_route = c_string_escape(&index.entry_route);
        let deep_link = super::native_deep_link::NativeDeepLink::c_helper(&application.links);
        let mut values = vec![
            ("__APP_NAME__", app_name.as_str()),
            ("__BUNDLE_ID__", bundle_id.as_str()),
            ("__ENTRY_ROUTE__", entry_route.as_str()),
            ("__NATIVE_MENU_ITEMS__", menu_items.as_str()),
            (
                "__EXTERNAL_LINK_HELPER__",
                super::native_external_link::NativeExternalLink::C_HELPER,
            ),
            ("__DEEP_LINK_HELPER__", deep_link.as_str()),
        ];
        values.extend_from_slice(platform_values);
        super::placeholder_fill::PlaceholderFill::once(&framed, &values)
    }

    fn frame(&self, template: &str, application: &NativeApplication) -> String {
        template
            .replace(
                "__NATIVE_SHIM__",
                &c_string_escape(&super::host::native_shim(&application.window, self.target)),
            )
            .replace(
                "__CONTENT_SECURITY_POLICY__",
                &c_string_escape(&application.content_security_policy()),
            )
            .replace("__NATIVE_MENUS__", self.menus)
    }
}
