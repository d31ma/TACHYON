use std::path::Path;

pub(super) struct InteropRules {
    pub(super) extension: &'static str,
    pub(super) language: &'static str,
    pub(super) words: &'static [&'static str],
    pub(super) imports: &'static [&'static str],
    pub(super) directives: &'static [&'static str],
    pub(super) network: &'static [&'static str],
}

impl InteropRules {
    const CLIENT: &'static str = "client";
    const DOTNET_WORDS: &'static [&'static str] = &[
        "InteropServices.JavaScript",
        "JSImport",
        "JSExport",
        "JSHost",
    ];

    const ALL: [Self; 10] = [
        Self::words(
            "rs",
            "Rust",
            &["wasm_bindgen", "web_sys", "js_sys", "wasm_import_module"],
        ),
        Self {
            extension: "go",
            language: "Go",
            words: &[],
            imports: &["syscall/js", "net/http", "net/http/"],
            directives: &["//go:wasmimport"],
            network: &["net/http", "net/http/", "//go:wasmimport"],
        },
        Self::words("cs", "C#", Self::DOTNET_WORDS),
        Self::words("fs", "F#", Self::DOTNET_WORDS),
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
            directives: &[],
            network: &[],
        },
        Self::words(
            "kt",
            "Kotlin",
            &[
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
        ),
        Self::words(
            "swift",
            "Swift",
            &[
                "JavaScriptKit",
                "JavaScriptEventLoop",
                "JSObject",
                "JSValue",
                "JSClosure",
                "JSPromise",
                "_extern",
                "_expose",
            ],
        ),
        Self::words(
            "java",
            "Java",
            &[
                "org.teavm",
                "JSBody",
                "JSObject",
                "JSExport",
                "JSFunctor",
                "JSProperty",
                "JSMethod",
                "JSClass",
                "JSModule",
                "JSTopLevel",
                "JSByRef",
                "native",
            ],
        ),
        Self::words(
            "hpp",
            "C++",
            &[
                "emscripten",
                "EM_JS",
                "EM_ASM",
                "EM_ASYNC_JS",
                "EMSCRIPTEN_KEEPALIVE",
                "import_module",
                "import_name",
                "export_name",
            ],
        ),
        Self::words("rb", "Ruby", &["TachyonWeb", "JS.", "JS::"]),
    ];

    const fn words(
        extension: &'static str,
        language: &'static str,
        words: &'static [&'static str],
    ) -> Self {
        Self {
            extension,
            language,
            words,
            imports: &[],
            directives: &[],
            network: &[],
        }
    }

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
