use super::host::run_tool;
use crate::Failure;
use crate::failure::diagnostic;
use std::path::Path;

pub(super) struct RustStandardLibrary;

impl RustStandardLibrary {
    pub(super) async fn require(target: &str, needed_by: &str) -> Result<(), Failure> {
        let directory = run_tool("rustc", &["--print", "target-libdir", "--target", target])
            .await
            .unwrap_or_default();
        Path::new(directory.trim())
            .is_dir()
            .then_some(())
            .ok_or_else(|| Self::missing(target, needed_by))
    }

    fn missing(target: &str, needed_by: &str) -> Failure {
        Failure::one(diagnostic(
            1605,
            format!(
                "{needed_by} needs the Rust standard library for {target}, which is not installed."
            ),
            Some(format!("Run rustup target add {target}, then build again.")),
            None,
        ))
    }
}
