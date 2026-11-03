use super::host::{NativeHost, native_tool_failure, run_tool_with};
use crate::Failure;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct GoCompiler {
    pub(super) name: &'static str,
    program: &'static str,
    arguments: &'static [&'static str],
    root_arguments: [&'static str; 2],
    glue_path: &'static str,
    glue_changes: &'static [(&'static str, &'static str)],
    pub(super) optimizer_flags: &'static [&'static str],
}

impl GoCompiler {
    const FAILURE_CODE: u16 = 1605;
    pub(super) const OUTPUT: &'static str = "services.wasm";
    const LICENSE_FILE: &'static str = "LICENSE";
    pub(super) const PINNED_GO_LICENSE: &'static str =
        include_str!("../../../../runtimes/go-web/LICENSE");
    const ENVIRONMENT: [(&'static str, &'static str); 8] = [
        ("GO111MODULE", "on"),
        ("GOFLAGS", ""),
        ("GOPROXY", "off"),
        ("GOTOOLCHAIN", "local"),
        ("GOWORK", "off"),
        ("CGO_ENABLED", "0"),
        ("GOOS", "js"),
        ("GOARCH", "wasm"),
    ];
    const TINYGO_SYNCHRONOUS_SLEEP: [(&'static str, &'static str); 3] = [
        (
            "\"runtime.sleepTicks\": (timeout) => {\n",
            "\"runtime.sleepTicks\": (timeout) => {\n\t\t\t\t\t\tthis._sleepUntil = performance.now() + Number(timeout) / 1e6;\n",
        ),
        (
            "\t\t\t\tgo._resume();\n\t\t\t\treturn event.result;",
            "\t\t\t\tgo._sleepUntil = undefined;\n\t\t\t\tgo._resume();\n\t\t\t\tgo._finishSleepingEvent(event);\n\t\t\t\treturn event.result;",
        ),
        (
            "\t\t_makeFuncWrapper(id) {",
            "\t\t_finishSleepingEvent(event) {\n\t\t\twhile (!('result' in event) && this._sleepUntil !== undefined && !this.exited) {\n\t\t\t\tconst until = this._sleepUntil;\n\t\t\t\tthis._sleepUntil = undefined;\n\t\t\t\twhile (performance.now() < until) {}\n\t\t\t\ttry {\n\t\t\t\t\tthis._inst.exports.go_scheduler();\n\t\t\t\t} catch (e) {\n\t\t\t\t\tif (e !== wasmExit) throw e;\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\n\t\t_makeFuncWrapper(id) {",
        ),
    ];
    pub(super) const TINYGO: Self = Self {
        name: "TinyGo",
        program: "tinygo",
        arguments: &[
            "build",
            "-target",
            "wasm",
            "-opt=z",
            "-no-debug",
            "-gc=boehm",
            "-stack-size=512KB",
            "-o",
            Self::OUTPUT,
            ".",
        ],
        root_arguments: ["env", "TINYGOROOT"],
        glue_path: "targets/wasm_exec.js",
        glue_changes: &Self::TINYGO_SYNCHRONOUS_SLEEP,
        optimizer_flags: &["-Oz", "--strip-debug", "--strip-producers"],
    };
    pub(super) const STANDARD: Self = Self {
        name: "the standard Go compiler",
        program: "go",
        arguments: &[
            "build",
            "-trimpath",
            "-buildvcs=false",
            "-ldflags=-s -w",
            "-o",
            Self::OUTPUT,
            ".",
        ],
        root_arguments: ["env", "GOROOT"],
        glue_path: "lib/wasm/wasm_exec.js",
        glue_changes: &[],
        optimizer_flags: &[
            "-O2",
            "--enable-sign-ext",
            "--enable-nontrapping-float-to-int",
            "--enable-bulk-memory",
            "--enable-bulk-memory-opt",
        ],
    };

    pub(super) async fn version(self) -> Result<String, Failure> {
        run_tool_with(self.program, &["version"], None, &Self::ENVIRONMENT).await
    }

    pub(super) async fn build(self, staged: &Path) -> Result<(), Failure> {
        NativeHost::run_limited(
            self.program,
            self.arguments,
            Some(staged),
            &Self::ENVIRONMENT,
            NativeHost::LONG_TOOL_TIME_LIMIT,
        )
        .await?;
        Ok(())
    }

    pub(super) async fn glue_and_license(self) -> Result<(Vec<u8>, Vec<u8>), Failure> {
        let root =
            run_tool_with(self.program, &self.root_arguments, None, &Self::ENVIRONMENT).await?;
        let root = Path::new(root.trim());
        let path = root.join(self.glue_path);
        let original = std::fs::read_to_string(&path).map_err(|error| {
            self.glue_failure(&format!("cannot read {}: {error}", path.display()))
        })?;
        let glue = self.changed(original)?.into_bytes();
        Ok((glue, Self::license(root)))
    }

    pub(super) fn license(root: &Path) -> Vec<u8> {
        let path = root.join(Self::LICENSE_FILE);
        std::fs::read(path).unwrap_or_else(|_| Self::PINNED_GO_LICENSE.as_bytes().to_vec())
    }

    fn changed(self, original: String) -> Result<String, Failure> {
        let mut glue = original;
        for (before, after) in self.glue_changes {
            glue = self.replaced(&glue, before, after)?;
        }
        Ok(glue)
    }

    fn replaced(self, glue: &str, before: &str, after: &str) -> Result<String, Failure> {
        if !glue.contains(before) {
            return Err(self.glue_failure(&format!(
                "it no longer contains `{}`; install the {} version that Tachyon supports (run `ty doctor`)",
                before.trim(),
                self.name
            )));
        }
        Ok(glue.replacen(before, after, 1))
    }

    fn glue_failure(self, detail: &str) -> Failure {
        native_tool_failure(
            Self::FAILURE_CODE,
            &format!(
                "Cannot prepare the WebAssembly glue of {}: {detail}",
                self.name
            ),
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::GoCompiler;

    #[test]
    fn the_license_beside_the_glue_comes_from_the_toolchain_root() {
        let root = tempfile::tempdir().expect("toolchain root");
        std::fs::write(root.path().join("LICENSE"), "The TinyGo Authors").expect("license");
        assert_eq!(GoCompiler::license(root.path()), b"The TinyGo Authors");
    }

    #[test]
    fn a_toolchain_root_without_a_license_ships_the_pinned_go_license() {
        let root = tempfile::tempdir().expect("toolchain root");
        let license = GoCompiler::license(root.path());
        assert_eq!(license, GoCompiler::PINNED_GO_LICENSE.as_bytes());
        let text = String::from_utf8(license).expect("text");
        assert!(text.starts_with("Copyright 2009 The Go Authors."));
        assert!(text.contains("Redistribution and use in source and binary forms"));
    }

    #[test]
    fn tinygo_builds_with_the_boehm_collector_so_calls_do_not_pause_for_milliseconds() {
        assert!(GoCompiler::TINYGO.arguments.contains(&"-gc=boehm"));
        assert!(!GoCompiler::STANDARD.arguments.contains(&"-gc=boehm"));
    }
}
