use super::network_words::NetworkWords;
use std::path::Path;

pub(super) struct InteropRules {
    pub(super) extension: &'static str,
    pub(super) language: &'static str,
    pub(super) words: &'static [&'static str],
    pub(super) imports: &'static [&'static str],
    pub(super) directives: &'static [&'static str],
    pub(super) network: NetworkWords,
}

impl InteropRules {
    const CLIENT: &'static str = "client";
    const DOTNET_WORDS: &'static [&'static str] = &[
        "InteropServices.JavaScript",
        "JSImport",
        "JSExport",
        "JSHost",
    ];
    const DOTNET_NETWORK: NetworkWords = NetworkWords::words(&[
        "System.Net.Http",
        "HttpClient",
        "HttpWebRequest",
        "WebRequest",
        "WebClient",
    ]);
    const JVM_NETWORK: &'static [&'static str] = &[
        "java.net.http",
        "HttpClient",
        "HttpURLConnection",
        "URLConnection",
        "okhttp3",
        "io.ktor",
    ];

    const ALL: [Self; 11] = [
        Self::words(
            "rs",
            "Rust",
            &["wasm_bindgen", "web_sys", "js_sys", "wasm_import_module"],
        )
        .with_network(NetworkWords::words(&[
            "reqwest",
            "ureq",
            "gloo_net",
            "ehttp",
            "isahc",
            "attohttpc",
            "surf::",
            "hyper::",
        ])),
        Self {
            extension: "go",
            language: "Go",
            words: &[],
            imports: &["syscall/js"],
            directives: &["//go:wasmimport"],
            network: NetworkWords::imports(&["net/http", "net/http/"]),
        },
        Self::words("cs", "C#", Self::DOTNET_WORDS).with_network(Self::DOTNET_NETWORK),
        Self::words("fs", "F#", Self::DOTNET_WORDS).with_network(Self::DOTNET_NETWORK),
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
            network: NetworkWords::imports(&["package:http/", "package:dio/", "dart:io"]),
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
        )
        .with_network(NetworkWords::words(Self::JVM_NETWORK)),
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
        )
        .with_network(NetworkWords::words(&[
            "URLSession",
            "URLRequest",
            "FoundationNetworking",
            "AsyncHTTPClient",
        ])),
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
        )
        .with_network(NetworkWords::words(Self::JVM_NETWORK)),
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
        )
        .with_network(NetworkWords::words(&[
            "emscripten_fetch",
            "emscripten/fetch.h",
            "curl_easy_init",
            "cpr::",
            "httplib::",
        ])),
        Self::words("rb", "Ruby", &["TachyonWeb", "JS.", "JS::"]).with_network(NetworkWords {
            words: &[
                "Net::HTTP",
                "Faraday",
                "HTTParty",
                "RestClient",
                "Excon",
                "URI.open",
            ],
            imports: &["net/http", "open-uri"],
        }),
        Self::words(
            "py",
            "Python",
            &[
                "import js",
                "from js",
                "import pyodide.ffi",
                "from pyodide.ffi",
            ],
        )
        .with_network(NetworkWords::words(&[
            "from js import fetch",
            "from js import XMLHttpRequest",
            "js.fetch",
            "js.XMLHttpRequest",
            "import pyodide.http",
            "from pyodide.http",
            "pyfetch",
            "import pyodide_http",
            "from pyodide_http",
            "import requests",
            "from requests",
            "import httpx",
            "from httpx",
            "import aiohttp",
            "from aiohttp",
            "import urllib.request",
            "from urllib.request",
            "from urllib import request",
            "import urllib3",
            "from urllib3",
            "import http.client",
            "from http.client",
            "from http import client",
        ])),
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
            network: NetworkWords::NONE,
        }
    }

    const fn with_network(self, network: NetworkWords) -> Self {
        Self { network, ..self }
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
