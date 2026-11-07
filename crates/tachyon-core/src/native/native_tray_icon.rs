use super::native_tray::NativeTray;
use crate::Failure;
use std::path::Path;

pub(super) struct NativeTrayIcon;

impl NativeTrayIcon {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    const MAX_BYTES: u64 = 1_048_576;
    const MIN_EDGE: u32 = 16;
    const MAX_EDGE: u32 = 1024;
    pub(super) const WINDOWS_FILE: &'static str = "tray.ico";

    pub(super) fn check(tray: Option<&NativeTray>, web_bundle: &Path) -> Result<(), Failure> {
        tray.map_or(Ok(()), |tray| {
            Self::require_png(tray, web_bundle).map(|_| ())
        })
    }

    pub(super) fn stage_for_windows(
        tray: Option<&NativeTray>,
        web_bundle: &Path,
        resources: &Path,
    ) -> Result<(), Failure> {
        tray.map_or(Ok(()), |tray| {
            Self::write_windows_icon(&Self::require_png(tray, web_bundle)?, resources)
        })
    }

    fn write_windows_icon(png: &[u8], resources: &Path) -> Result<(), Failure> {
        let length = u32::try_from(png.len()).unwrap_or(u32::MAX);
        super::host::write(
            &resources.join(Self::WINDOWS_FILE),
            &super::windows::ico_bytes(png, length),
        )
    }

    fn require_png(tray: &NativeTray, web_bundle: &Path) -> Result<Vec<u8>, Failure> {
        Self::read(&web_bundle.join(tray.icon_path()))
            .filter(|bytes| Self::edges(bytes).is_some_and(Self::fits))
            .ok_or_else(|| {
                super::config::config_failure(&format!(
                    "Manifest tac.application.tray.icon '{}' must be a PNG file of at most 1 MiB in the built client bundle, from 16 to 1024 pixels on each edge.",
                    tray.icon
                ))
            })
    }

    fn read(path: &Path) -> Option<Vec<u8>> {
        std::fs::symlink_metadata(path)
            .ok()
            .filter(|metadata| [metadata.is_file(), metadata.len() <= Self::MAX_BYTES] == [true; 2])
            .and_then(|_| std::fs::read(path).ok())
    }

    fn edges(bytes: &[u8]) -> Option<(u32, u32)> {
        let header = bytes.get(..24)?;
        let png = [header[..8] == Self::SIGNATURE, &header[12..16] == b"IHDR"] == [true; 2];
        png.then(|| (Self::number(&header[16..20]), Self::number(&header[20..24])))
    }

    fn number(bytes: &[u8]) -> u32 {
        u32::from_be_bytes(bytes.try_into().unwrap_or([0; 4]))
    }

    fn fits(edges: (u32, u32)) -> bool {
        [edges.0, edges.1]
            .iter()
            .all(|edge| (Self::MIN_EDGE..=Self::MAX_EDGE).contains(edge))
    }
}
