use super::go_compiler::GoCompiler;

pub(super) struct GoCompiledWith {
    pub(super) compiler: GoCompiler,
    pub(super) reason: String,
}

impl GoCompiledWith {
    const SHRUNK: &'static str = ", then Binaryen wasm-opt";
    const NOT_SHRUNK: &'static str =
        "; install Binaryen or run `ty doctor` to make it smaller still";

    pub(super) fn note(&self, shrunk: bool) -> String {
        format!(
            "Go: {} ({}){}",
            self.compiler.name,
            self.reason,
            Self::optimizer_text(shrunk)
        )
    }

    const fn optimizer_text(shrunk: bool) -> &'static str {
        if shrunk {
            Self::SHRUNK
        } else {
            Self::NOT_SHRUNK
        }
    }
}
