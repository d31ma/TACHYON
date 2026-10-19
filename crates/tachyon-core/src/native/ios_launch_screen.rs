use super::config::WindowConfiguration;
use super::host::{native_io, run_tool, write};
use super::native_theme_color::NativeThemeColor;
use super::system_version::SystemVersion;
use crate::Failure;
use std::path::Path;

pub(super) struct IosLaunchScreen;

impl IosLaunchScreen {
    const COLOR_NAME: &'static str = "TachyonLaunchBackground";
    const CATALOG: &'static str = "LaunchScreen.xcassets";
    const CATALOG_CONTENTS: &'static str = r#"{"info":{"author":"xcode","version":1}}"#;

    pub(super) fn plist_entry(window: &WindowConfiguration) -> String {
        let color = window.background_color.map_or_else(String::new, |_| {
            format!(
                "<key>UIColorName</key><string>{}</string>",
                Self::COLOR_NAME
            )
        });
        format!("<key>UILaunchScreen</key><dict>{color}</dict>")
    }

    pub(super) fn stage_color(catalog: &Path, window: &WindowConfiguration) -> Result<(), Failure> {
        let Some(color) = window.background_color else {
            return Ok(());
        };
        let color_set = catalog.join(format!("{}.colorset", Self::COLOR_NAME));
        native_io(std::fs::create_dir_all(&color_set), &color_set)?;
        write(
            &color_set.join("Contents.json"),
            Self::color_contents(color).as_bytes(),
        )
    }

    pub(super) async fn compile(
        window: &WindowConfiguration,
        scratch: &Path,
        bundle: &Path,
        (platform, deployment): (&str, SystemVersion),
    ) -> Result<(), Failure> {
        if window.background_color.is_none() {
            return Ok(());
        }
        let catalog = scratch.join(Self::CATALOG);
        native_io(std::fs::create_dir_all(&catalog), &catalog)?;
        write(
            &catalog.join("Contents.json"),
            Self::CATALOG_CONTENTS.as_bytes(),
        )?;
        Self::stage_color(&catalog, window)?;
        run_tool(
            "/usr/bin/xcrun",
            &[
                "actool",
                "--compile",
                &bundle.to_string_lossy(),
                "--platform",
                platform,
                "--minimum-deployment-target",
                &deployment.to_string(),
                "--target-device",
                "iphone",
                "--target-device",
                "ipad",
                "--errors",
                "--warnings",
                "--output-format",
                "human-readable-text",
                &catalog.to_string_lossy(),
            ],
        )
        .await
        .map(|_| ())
    }

    fn color_contents(color: NativeThemeColor) -> String {
        format!(
            r#"{{"colors":[{{"color":{{"color-space":"srgb","components":{{"alpha":"1.000","red":"0x{:02X}","green":"0x{:02X}","blue":"0x{:02X}"}}}},"idiom":"universal"}}],"info":{{"author":"xcode","version":1}}}}"#,
            color.red, color.green, color.blue
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{IosLaunchScreen, WindowConfiguration};
    use crate::native::native_theme_color::NativeThemeColor;

    fn colored() -> WindowConfiguration {
        WindowConfiguration {
            background_color: Some(NativeThemeColor {
                red: 0x0f,
                green: 0x6c,
                blue: 0xbd,
            }),
            ..WindowConfiguration::default()
        }
    }

    #[test]
    fn the_launch_screen_names_the_background_color_only_when_the_manifest_has_one() {
        assert_eq!(
            IosLaunchScreen::plist_entry(&WindowConfiguration::default()),
            "<key>UILaunchScreen</key><dict></dict>"
        );
        assert_eq!(
            IosLaunchScreen::plist_entry(&colored()),
            "<key>UILaunchScreen</key><dict><key>UIColorName</key><string>TachyonLaunchBackground</string></dict>"
        );
    }

    #[test]
    fn the_color_set_holds_the_manifest_channels() {
        let catalog = tempfile::tempdir().expect("catalog");
        IosLaunchScreen::stage_color(catalog.path(), &WindowConfiguration::default())
            .expect("no color");
        assert!(
            std::fs::read_dir(catalog.path())
                .expect("read")
                .next()
                .is_none()
        );
        IosLaunchScreen::stage_color(catalog.path(), &colored()).expect("color");
        let contents = std::fs::read_to_string(
            catalog
                .path()
                .join("TachyonLaunchBackground.colorset/Contents.json"),
        )
        .expect("contents");
        let parsed: serde_json::Value = serde_json::from_str(&contents).expect("json");
        assert_eq!(
            parsed["colors"][0]["color"]["components"],
            serde_json::json!({ "alpha": "1.000", "red": "0x0F", "green": "0x6C", "blue": "0xBD" })
        );
    }
}
