use std::path::Path;
use tachyon_core::{AppTarget, BuildResult, Failure, TargetSupport};

pub(crate) struct WebBuildReport;

impl WebBuildReport {
    pub(crate) fn print(project: &Path, result: &BuildResult) -> Result<(), Failure> {
        println!(
            "Built {} {} to {} ({}) compiled={} reused={}",
            result.route_count(),
            Self::noun(result.route_count()),
            result.output_directory().display(),
            result.sha256(),
            result.compiled_routes(),
            result.reused_routes()
        );
        Self::print_compression(result);
        Self::print_lines(result.service_notes());
        Self::print_lines(&TargetSupport::notes(project, AppTarget::Web)?);
        Ok(())
    }

    fn noun(route_count: usize) -> &'static str {
        if route_count == 1 {
            return "route";
        }
        "routes"
    }

    fn print_compression(result: &BuildResult) {
        if let Some(line) = result.compression().line() {
            println!("{line}");
        }
    }

    fn print_lines(lines: &[String]) {
        for line in lines {
            println!("{line}");
        }
    }
}
