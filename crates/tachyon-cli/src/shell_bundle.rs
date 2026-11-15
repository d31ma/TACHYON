use crate::BuildTarget;
use crate::web_build_report::WebBuildReport;
use std::path::Path;
use tachyon_core::{BuildOptions, Failure, WebCompiler};

pub(crate) struct ShellBundle;

impl ShellBundle {
    const TARGET_CODE: u16 = 1001;

    pub(crate) async fn execute(
        project: &Path,
        out_dir: &Path,
        targets: &[BuildTarget],
        no_incremental: bool,
    ) -> Result<(), Failure> {
        Self::check_targets(targets)?;
        let options = BuildOptions {
            output_directory: out_dir.to_path_buf(),
            incremental: !no_incremental,
            ..BuildOptions::default()
        };
        let result = WebCompiler::build_shell_async(project, &options).await?;
        WebBuildReport::print(project, &result)
    }

    fn check_targets(targets: &[BuildTarget]) -> Result<(), Failure> {
        if targets == [BuildTarget::Web] {
            return Ok(());
        }
        Err(crate::cli_failure(
            Self::TARGET_CODE,
            "bundle --shell-only builds the web shell only.",
            "Drop --target, or use --target web.",
        ))
    }
}
