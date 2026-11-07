use super::config::NativeApplication;
use super::desktop_menu_items::DesktopMenuItems;
use super::native_tray::NativeTray;

pub(super) struct DesktopTraySource;

impl DesktopTraySource {
    const ABSENT: &'static str = "#define TACHYON_TRAY 0\n";

    pub(super) fn c_settings(
        application: &NativeApplication,
        mnemonic: char,
        quit: &str,
    ) -> String {
        application.tray.as_ref().map_or_else(
            || Self::ABSENT.to_owned(),
            |tray| Self::declared(tray, &application.name, mnemonic, quit),
        )
    }

    fn declared(tray: &NativeTray, name: &str, mnemonic: char, quit: &str) -> String {
        let label = name.replace(mnemonic, &format!("{mnemonic}{mnemonic}"));
        format!(
            "#define TACHYON_TRAY 1\n#define TACHYON_TRAY_CLOSE_HIDES {}\n#define TACHYON_TRAY_ICON {}\n#define TACHYON_TRAY_ICON_FILE L\"{}\"\n#define TACHYON_TRAY_TOOLTIP {}\n#define TACHYON_TRAY_OPEN_LABEL {}\n#define TACHYON_TRAY_QUIT_LABEL {}\n",
            u8::from(tray.close_hides_window),
            DesktopMenuItems::quoted(tray.icon_path()),
            super::native_tray_icon::NativeTrayIcon::WINDOWS_FILE,
            DesktopMenuItems::quoted(tray.tooltip_or(name)),
            DesktopMenuItems::quoted(&format!("Open {label}")),
            DesktopMenuItems::quoted(&format!("{quit} {label}")),
        )
    }
}
