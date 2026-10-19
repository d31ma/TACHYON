use super::config::{NativeApplication, WindowConfiguration};
use super::host::{native_io, run_tool, write};
use super::ios_launch_screen::IosLaunchScreen;
use super::system_version::SystemVersion;
use crate::Failure;
use crate::failure::diagnostic;
use std::path::{Path, PathBuf};

pub(super) struct IosAppIcon;

impl IosAppIcon {
    const CATALOG: [&'static str; 3] = ["Assets.xcassets", "AppIcon.appiconset", "AppIcon.png"];
    const ICON_NAME: &'static str = "AppIcon";
    const EDGE: &'static str = "1024";
    const PARTIAL_PLIST: &'static str = "icon-info.plist";
    const CATALOG_CONTENTS: &'static str = r#"{"info":{"author":"xcode","version":1}}"#;
    const ICON_CONTENTS: &'static str = r#"{"images":[{"filename":"AppIcon.png","idiom":"universal","platform":"ios","size":"1024x1024"}],"info":{"author":"xcode","version":1}}"#;
    const TRANSPARENT: &'static str = "hasAlpha: yes";

    pub(super) fn source(application: &NativeApplication, web_bundle: &Path) -> Option<PathBuf> {
        application
            .largest_icon()
            .map(|source| web_bundle.join(source.trim_start_matches('/')))
            .filter(|path| path.is_file())
    }

    pub(super) fn require_for_store(source: Option<&Path>, app_store: bool) -> Result<(), Failure> {
        let missing = [app_store, source.is_none()] == [true, true];
        (!missing).then_some(()).ok_or_else(|| {
            Self::refusal(
                "App Store Connect needs an app icon, but client/manifest.json names no PNG or JPEG icon that the app bundles.",
            )
        })
    }

    pub(super) async fn plist_keys(
        (source, window): (Option<&Path>, &WindowConfiguration),
        bundle: &Path,
        scratch: &Path,
        deployment: SystemVersion,
        app_store: bool,
    ) -> Result<String, Failure> {
        Self::compiled((source, window), bundle, scratch, deployment, app_store).await
    }

    async fn compiled(
        (source, window): (Option<&Path>, &WindowConfiguration),
        bundle: &Path,
        scratch: &Path,
        deployment: SystemVersion,
        app_store: bool,
    ) -> Result<String, Failure> {
        let Some(source) = source else {
            return Ok(String::new());
        };
        let catalog = scratch.join(Self::CATALOG[0]);
        let icon = Self::stage_catalog(&catalog, source).await?;
        IosLaunchScreen::stage_color(&catalog, window)?;
        Self::require_opaque(&icon, app_store).await?;
        Self::compile_catalog(&catalog, bundle, scratch, deployment).await
    }

    pub(super) async fn stage_catalog(catalog: &Path, source: &Path) -> Result<PathBuf, Failure> {
        let icon_set = catalog.join(Self::CATALOG[1]);
        native_io(std::fs::create_dir_all(&icon_set), &icon_set)?;
        write(
            &catalog.join("Contents.json"),
            Self::CATALOG_CONTENTS.as_bytes(),
        )?;
        write(
            &icon_set.join("Contents.json"),
            Self::ICON_CONTENTS.as_bytes(),
        )?;
        let icon = icon_set.join(Self::CATALOG[2]);
        run_tool(
            "/usr/bin/sips",
            &[
                "-z",
                Self::EDGE,
                Self::EDGE,
                &source.to_string_lossy(),
                "--out",
                &icon.to_string_lossy(),
            ],
        )
        .await?;
        Ok(icon)
    }

    pub(super) async fn require_opaque(icon: &Path, app_store: bool) -> Result<(), Failure> {
        let output = run_tool(
            "/usr/bin/sips",
            &["-g", "hasAlpha", &icon.to_string_lossy()],
        )
        .await?;
        Self::require_opaque_output(&output, app_store)
    }

    pub(super) fn require_opaque_output(output: &str, app_store: bool) -> Result<(), Failure> {
        let transparent = [app_store, output.contains(Self::TRANSPARENT)] == [true, true];
        (!transparent).then_some(()).ok_or_else(|| {
            Self::refusal(
                "App Store Connect refuses an app icon with an alpha channel, and the largest icon in client/manifest.json has one.",
            )
        })
    }

    pub(super) async fn compile_catalog(
        catalog: &Path,
        bundle: &Path,
        scratch: &Path,
        deployment: SystemVersion,
    ) -> Result<String, Failure> {
        let partial = scratch.join(Self::PARTIAL_PLIST);
        let partial_text = partial.to_string_lossy();
        run_tool(
            "/usr/bin/xcrun",
            &[
                "actool",
                "--compile",
                &bundle.to_string_lossy(),
                "--platform",
                "iphoneos",
                "--minimum-deployment-target",
                &deployment.to_string(),
                "--app-icon",
                Self::ICON_NAME,
                "--output-partial-info-plist",
                &partial_text,
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
        .await?;
        run_tool("/usr/bin/plutil", &["-convert", "xml1", &partial_text]).await?;
        let text = native_io(std::fs::read_to_string(&partial), &partial)?;
        Ok(Self::dictionary_keys(&text))
    }

    pub(super) fn dictionary_keys(plist: &str) -> String {
        plist
            .split_once("<dict>")
            .and_then(|(_, rest)| rest.rsplit_once("</dict>"))
            .map_or_else(String::new, |(keys, _)| format!("\n{}", keys.trim()))
    }

    fn refusal(message: &str) -> Failure {
        Failure::one(diagnostic(
            1601,
            message,
            Some(String::from(
                "Add a square PNG icon of at least 1024x1024 pixels without transparency to icons in client/manifest.json, or build with --device instead of --release.",
            )),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::IosAppIcon;
    use std::path::Path;

    #[test]
    fn a_store_build_needs_an_opaque_icon() {
        let icon = Path::new("icon.png");
        IosAppIcon::require_for_store(Some(icon), true).expect("icon");
        IosAppIcon::require_for_store(None, false).expect("device builds may skip the icon");
        let text = IosAppIcon::require_for_store(None, true)
            .expect_err("no icon")
            .to_string();
        assert!(text.contains("TY1601"), "{text}");
        assert!(text.contains("needs an app icon"), "{text}");

        let transparent = "/tmp/AppIcon.png\n  hasAlpha: yes\n";
        IosAppIcon::require_opaque_output(transparent, false).expect("device builds allow alpha");
        IosAppIcon::require_opaque_output("  hasAlpha: no\n", true).expect("opaque");
        let text = IosAppIcon::require_opaque_output(transparent, true)
            .expect_err("alpha")
            .to_string();
        assert!(text.contains("alpha channel"), "{text}");
    }

    #[test]
    fn the_compiled_icon_keys_join_the_info_plist() {
        let partial = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\">\n<dict>\n\t<key>CFBundleIcons</key>\n\t<dict>\n\t\t<key>CFBundlePrimaryIcon</key>\n\t\t<dict>\n\t\t\t<key>CFBundleIconName</key>\n\t\t\t<string>AppIcon</string>\n\t\t</dict>\n\t</dict>\n</dict>\n</plist>\n";
        let keys = IosAppIcon::dictionary_keys(partial);
        assert!(keys.starts_with("\n<key>CFBundleIcons</key>"), "{keys}");
        assert!(keys.contains("<string>AppIcon</string>"));
        assert!(keys.trim_end().ends_with("</dict>"));
        assert_eq!(
            keys.matches("<dict>").count(),
            keys.matches("</dict>").count()
        );
        assert_eq!(IosAppIcon::dictionary_keys("<plist/>"), "");
    }
}
