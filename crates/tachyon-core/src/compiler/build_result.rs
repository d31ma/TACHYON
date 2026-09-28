use super::CompressionTotals;
use super::site_build::SiteBuild;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildResult {
    pub(super) output_directory: PathBuf,
    pub(super) route_count: usize,
    pub(super) sha256: String,
    pub(super) compiled_routes: usize,
    pub(super) reused_routes: usize,
    pub(super) timings: Vec<(String, Duration)>,
    pub(super) service_timings: Vec<(String, Duration)>,
    pub(super) service_notes: Vec<String>,
    pub(super) compression: CompressionTotals,
}

impl BuildResult {
    #[must_use]
    pub fn output_directory(&self) -> &Path {
        &self.output_directory
    }

    #[must_use]
    pub const fn route_count(&self) -> usize {
        self.route_count
    }

    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    #[must_use]
    pub const fn compiled_routes(&self) -> usize {
        self.compiled_routes
    }

    #[must_use]
    pub const fn reused_routes(&self) -> usize {
        self.reused_routes
    }

    #[must_use]
    pub fn timings(&self) -> &[(String, Duration)] {
        &self.timings
    }

    #[must_use]
    pub fn service_timings(&self) -> &[(String, Duration)] {
        &self.service_timings
    }

    #[must_use]
    pub fn service_notes(&self) -> &[String] {
        &self.service_notes
    }

    #[must_use]
    pub const fn compression(&self) -> CompressionTotals {
        self.compression
    }

    #[must_use]
    pub fn timings_line(&self) -> String {
        let total: Duration = self.timings.iter().map(|(_, took)| *took).sum();
        let phases: Vec<String> = self
            .timings
            .iter()
            .map(|(phase, took)| self.phase_text(phase, *took))
            .collect();
        let mut line = format!(
            "Build took {:.1}s: {}",
            total.as_secs_f64(),
            phases.join(", ")
        );
        for note in &self.service_notes {
            line.push('\n');
            line.push_str(note);
        }
        line
    }

    fn phase_text(&self, phase: &str, took: Duration) -> String {
        let details = self.service_details(phase);
        format!("{phase} {:.1}s{details}", took.as_secs_f64())
    }

    fn service_details(&self, phase: &str) -> String {
        if phase != SiteBuild::SERVICES_PHASE || self.service_timings.is_empty() {
            return String::new();
        }
        let languages: Vec<String> = self
            .service_timings
            .iter()
            .map(|(language, took)| format!("{language} {:.1}s", took.as_secs_f64()))
            .collect();
        format!(" ({})", languages.join(", "))
    }
}
