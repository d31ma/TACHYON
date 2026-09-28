use std::path::Path;
use tachyon_core::Failure;
use tachyon_core::method_coverage::{CoverageReport, CoverageText, RecordedCalls};

pub(crate) struct CoverageGate;

impl CoverageGate {
    const UNTESTED: u16 = 2023;

    pub(crate) fn print_test_report(
        project: &Path,
        calls: RecordedCalls,
        strict: bool,
    ) -> Result<bool, Failure> {
        let report = CoverageReport::build(project, calls)?;
        Self::print(&report, strict);
        let has_uncovered = report.covered() < report.total();
        Ok([strict, has_uncovered] == [true, true])
    }

    pub(crate) fn refuse_untested(project: &Path, calls: RecordedCalls) -> Result<(), Failure> {
        let report = CoverageReport::build(project, calls)?;
        Self::verdict(&report)
    }

    fn print(report: &CoverageReport, strict: bool) {
        if report.total() == 0 {
            return;
        }
        Self::show(report, strict);
    }

    fn show(report: &CoverageReport, strict: bool) {
        println!("{}", CoverageText::summary(report));
        Self::print_lines(&Self::lines(report, strict));
    }

    fn print_lines(lines: &[String]) {
        for line in lines {
            println!("{line}");
        }
    }

    fn lines(report: &CoverageReport, strict: bool) -> Vec<String> {
        if strict {
            return CoverageText::details(report);
        }
        CoverageText::missing(report)
    }

    fn verdict(report: &CoverageReport) -> Result<(), Failure> {
        let uncovered = report.total() - report.covered();
        if uncovered == 0 {
            return Ok(());
        }
        Err(crate::cli_failure(
            Self::UNTESTED,
            format!(
                "{uncovered} of {} public methods on annotated classes were not called by any test case:\n{}",
                report.total(),
                CoverageText::missing(report).join("\n")
            ),
            "Call each listed method from a test* case and run ty test, drop the class-level annotation, or set TAC_ALLOW_UNTESTED=1 for this run.",
        ))
    }
}
