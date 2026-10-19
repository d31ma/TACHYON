use super::host::{NativeHost, native_tool_failure, run_tool};
use super::macos::unicode_path;
use crate::Failure;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MacOsDiskImage;

impl MacOsDiskImage {
    const APPLICATIONS: &'static str = "/Applications";

    pub(super) async fn create(volume: &str, app: &Path, dmg: &Path) -> Result<(), Failure> {
        let parent = dmg.parent().unwrap_or(app);
        let root = tempfile::Builder::new()
            .prefix(".tachyon-disk-image-")
            .tempdir_in(parent)
            .map_err(|error| {
                native_tool_failure(1605, &format!("Cannot stage the .dmg contents: {error}"))
            })?;
        let app_name = app.file_name().unwrap_or_default();
        let copied = root.path().join(app_name);
        let shortcut = root.path().join("Applications");
        let source = unicode_path(app, "Application path is not valid Unicode.")?;
        let copied = unicode_path(&copied, "Disk image path is not valid Unicode.")?;
        let shortcut = unicode_path(&shortcut, "Disk image path is not valid Unicode.")?;
        run_tool("/usr/bin/ditto", &[source, copied]).await?;
        run_tool("/bin/ln", &["-s", Self::APPLICATIONS, shortcut]).await?;
        let root_path = unicode_path(root.path(), "Disk image path is not valid Unicode.")?;
        let image = unicode_path(dmg, "Disk image path is not valid Unicode.")?;
        let volume = Self::volume_name(volume);
        let arguments = Self::arguments(&volume, root_path, image);
        NativeHost::run_long_tool("/usr/bin/hdiutil", &arguments).await?;
        Ok(())
    }

    pub(super) fn arguments<'value>(
        volume: &'value str,
        root: &'value str,
        dmg: &'value str,
    ) -> [&'value str; 10] {
        [
            "create",
            "-volname",
            volume,
            "-srcfolder",
            root,
            "-ov",
            "-format",
            "UDZO",
            "-quiet",
            dmg,
        ]
    }

    pub(super) fn volume_name(name: &str) -> String {
        name.replace(['/', ':'], "-")
    }
}

#[cfg(test)]
mod tests {
    use super::MacOsDiskImage;

    #[test]
    fn hdiutil_writes_a_compressed_image_of_the_app_and_an_applications_shortcut() {
        assert_eq!(
            MacOsDiskImage::arguments("Notes", "stage/root", "stage/Notes.dmg"),
            [
                "create",
                "-volname",
                "Notes",
                "-srcfolder",
                "stage/root",
                "-ov",
                "-format",
                "UDZO",
                "-quiet",
                "stage/Notes.dmg"
            ]
        );
    }

    #[test]
    fn a_volume_name_keeps_no_path_separator() {
        assert_eq!(MacOsDiskImage::volume_name("Notes: A/B"), "Notes- A-B");
    }
}
