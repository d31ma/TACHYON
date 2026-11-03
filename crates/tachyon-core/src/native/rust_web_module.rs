use crate::wasm_release_note::WasmReleaseNote;
use crate::wasm_release_step::WasmReleaseStep;

#[derive(Default)]
pub(crate) struct RustWebModule {
    pub(crate) binary: Vec<u8>,
    pub(crate) note: Option<String>,
}

impl RustWebModule {
    pub(super) const DEVELOPMENT_ARGUMENTS: [&'static str; 8] = [
        "-C",
        "opt-level=s",
        "-C",
        "lto",
        "-C",
        "panic=abort",
        "-C",
        "strip=symbols",
    ];
    pub(super) const RELEASE_ARGUMENTS: [&'static str; 10] = [
        "-C",
        "opt-level=z",
        "-C",
        "lto",
        "-C",
        "codegen-units=1",
        "-C",
        "panic=abort",
        "-C",
        "strip=symbols",
    ];
    pub(super) const OPTIMIZER_FLAGS: [&'static str; 12] = [
        "-Oz",
        "--converge",
        "--low-memory-unused",
        "--enable-bulk-memory",
        "--enable-bulk-memory-opt",
        "--enable-sign-ext",
        "--enable-nontrapping-float-to-int",
        "--enable-mutable-globals",
        "--enable-multivalue",
        "--enable-reference-types",
        "--strip-debug",
        "--strip-producers",
    ];
    const NOTE: WasmReleaseNote = WasmReleaseNote {
        optimized: "Rust: rustc tuned for size (opt-level z, LTO, one codegen unit), then Binaryen wasm-opt -Oz",
        unoptimized: "Rust: rustc tuned for size, but shipped without Binaryen wasm-opt (about 10% larger); install Binaryen or run `ty doctor`",
    };

    pub(super) fn built(binary: Vec<u8>, release: &WasmReleaseStep, shrunk: bool) -> Self {
        Self {
            binary,
            note: release.note(shrunk, &Self::NOTE),
        }
    }
}
