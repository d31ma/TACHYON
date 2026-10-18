use super::host::run_tool;
use crate::Failure;
use crate::failure::diagnostic;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MacOsSlice {
    name: &'static str,
    rust_target: &'static str,
}

impl MacOsSlice {
    pub(super) const ARM64: Self = Self {
        name: "arm64",
        rust_target: "aarch64-apple-darwin",
    };
    pub(super) const X64: Self = Self {
        name: "x86_64",
        rust_target: "x86_64-apple-darwin",
    };

    pub(super) const fn name(self) -> &'static str {
        self.name
    }

    pub(super) const fn rust_target(self) -> &'static str {
        self.rust_target
    }

    pub(super) fn swift_target(self, minimum_system_version: &str) -> String {
        format!("{}-apple-macosx{minimum_system_version}", self.name)
    }

    pub(super) async fn require_rust_target(self) -> Result<(), Failure> {
        let target = self.rust_target;
        let directory = run_tool("rustc", &["--print", "target-libdir", "--target", target])
            .await
            .unwrap_or_default();
        Path::new(directory.trim())
            .is_dir()
            .then_some(())
            .ok_or_else(|| self.missing_rust_target())
    }

    fn missing_rust_target(self) -> Failure {
        let target = self.rust_target;
        Failure::one(diagnostic(
            1605,
            format!(
                "The {} part of the macOS app needs the Rust standard library for {target}, which is not installed.",
                self.name
            ),
            Some(format!("Run rustup target add {target}, then build again.")),
            None,
        ))
    }
}
