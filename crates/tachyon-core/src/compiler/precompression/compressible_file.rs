use std::path::Path;

pub(super) struct CompressibleFile;

impl CompressibleFile {
    pub(super) const MIN_BYTES: usize = 1024;
    const EXTENSIONS: [&'static str; 12] = [
        "js",
        "mjs",
        "cjs",
        "css",
        "html",
        "json",
        "webmanifest",
        "svg",
        "wasm",
        "map",
        "xml",
        "txt",
    ];

    pub(super) fn accepts(path: &Path, bytes: &[u8]) -> bool {
        [
            bytes.len() >= Self::MIN_BYTES,
            Self::has_text_or_code_extension(path),
        ] == [true, true]
    }

    fn has_text_or_code_extension(path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                Self::EXTENSIONS
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(extension))
            })
    }
}
