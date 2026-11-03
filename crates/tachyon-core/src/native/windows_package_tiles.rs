use super::config::NativeApplication;
use super::host::{native_io, write};
use crate::Failure;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::io::Write;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct WindowsPackageTiles {
    file: &'static str,
    edge: u32,
}

impl WindowsPackageTiles {
    pub(super) const STORE_LOGO: Self = Self {
        file: "StoreLogo.png",
        edge: 50,
    };
    pub(super) const SQUARE_44: Self = Self {
        file: "Square44x44Logo.png",
        edge: 44,
    };
    pub(super) const SQUARE_150: Self = Self {
        file: "Square150x150Logo.png",
        edge: 150,
    };
    pub(super) const EVERY_TILE: [Self; 3] = [Self::STORE_LOGO, Self::SQUARE_44, Self::SQUARE_150];
    const FOLDER: &'static str = "Assets";
    const PLAIN_COLOR: [u8; 3] = [128, 128, 128];
    const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    const RGB_HEADER_TAIL: [u8; 5] = [8, 2, 0, 0, 0];

    pub(super) fn manifest_path(self) -> String {
        format!("{}\\{}", Self::FOLDER, self.file)
    }

    pub(super) fn stage_all(
        application: &NativeApplication,
        web_bundle: &Path,
        layout: &Path,
    ) -> Result<(), Failure> {
        Self::EVERY_TILE
            .into_iter()
            .try_for_each(|tile| tile.stage(application, web_bundle, layout))
    }

    fn stage(
        self,
        application: &NativeApplication,
        web_bundle: &Path,
        layout: &Path,
    ) -> Result<(), Failure> {
        let target = layout.join(Self::FOLDER).join(self.file);
        let chosen = self
            .icon_for(application)
            .map(|source| web_bundle.join(source.trim_start_matches('/')))
            .filter(|source| source.is_file());
        match chosen {
            Some(source) => write(&target, &native_io(std::fs::read(&source), &source)?),
            None => write(&target, &self.plain_tile(application)),
        }
    }

    pub(super) fn icon_for(self, application: &NativeApplication) -> Option<&str> {
        let portable: Vec<&(u32, String)> = application
            .icons
            .iter()
            .filter(|(_, source)| source.to_ascii_lowercase().ends_with(".png"))
            .collect();
        let large_enough = portable.iter().rev().find(|(edge, _)| *edge >= self.edge);
        large_enough
            .or_else(|| portable.first())
            .map(|(_, source)| source.as_str())
    }

    pub(super) fn plain_tile(self, application: &NativeApplication) -> Vec<u8> {
        let color = application
            .window
            .theme_color
            .map_or(Self::PLAIN_COLOR, |theme| {
                [theme.red, theme.green, theme.blue]
            });
        let row: Vec<u8> = std::iter::once(0)
            .chain(color.repeat(self.edge as usize))
            .collect();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(&row.repeat(self.edge as usize))
            .unwrap_or_default();
        let pixels = encoder.finish().unwrap_or_default();
        let mut header = Vec::with_capacity(13);
        header.extend_from_slice(&self.edge.to_be_bytes());
        header.extend_from_slice(&self.edge.to_be_bytes());
        header.extend_from_slice(&Self::RGB_HEADER_TAIL);
        let mut png = Self::PNG_SIGNATURE.to_vec();
        Self::chunk(&mut png, *b"IHDR", &header);
        Self::chunk(&mut png, *b"IDAT", &pixels);
        Self::chunk(&mut png, *b"IEND", &[]);
        png
    }

    fn chunk(png: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
        let length = u32::try_from(data.len()).unwrap_or(u32::MAX);
        png.extend_from_slice(&length.to_be_bytes());
        png.extend_from_slice(&kind);
        png.extend_from_slice(data);
        let mut crc = flate2::Crc::new();
        crc.update(&kind);
        crc.update(data);
        png.extend_from_slice(&crc.sum().to_be_bytes());
    }
}
