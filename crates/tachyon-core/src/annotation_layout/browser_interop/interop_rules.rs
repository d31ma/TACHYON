use std::path::Path;

pub(super) struct InteropRules {
    pub(super) extension: &'static str,
    pub(super) language: &'static str,
    pub(super) words: &'static [&'static str],
    pub(super) imports: &'static [&'static str],
}

impl InteropRules {
    const CLIENT: &'static str = "client";

    const ALL: [Self; 6] = [
        Self {
            extension: "rs",
            language: "Rust",
            words: &["wasm_bindgen", "web_sys", "js_sys", "wasm_import_module"],
            imports: &[],
        },
        Self {
            extension: "go",
            language: "Go",
            words: &[],
            imports: &["syscall/js"],
        },
        Self {
            extension: "cs",
            language: "C#",
            words: &[
                "InteropServices.JavaScript",
                "JSImport",
                "JSExport",
                "JSHost",
            ],
            imports: &[],
        },
        Self {
            extension: "dart",
            language: "Dart",
            words: &[],
            imports: &[
                "dart:js_interop",
                "dart:js_interop_unsafe",
                "dart:js_util",
                "dart:js",
                "dart:html",
                "package:web/",
                "package:js/",
            ],
        },
        Self {
            extension: "kt",
            language: "Kotlin",
            words: &[
                "external",
                "JsFun",
                "JsExport",
                "JsModule",
                "JsName",
                "js(",
                "kotlinx.browser",
                "org.w3c",
                "kotlin.js",
            ],
            imports: &[],
        },
        Self {
            extension: "rb",
            language: "Ruby",
            words: &["TachyonWeb", "JS.", "JS::"],
            imports: &[],
        },
    ];

    pub(super) fn for_path(path: &Path) -> Option<&'static Self> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let rules = Self::for_extension(extension);
        rules.filter(|_| path.starts_with(Self::CLIENT))
    }

    fn for_extension(extension: &str) -> Option<&'static Self> {
        Self::ALL.iter().find(|rules| rules.extension == extension)
    }
}
