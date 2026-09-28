use super::{ClassCoverage, CoverageReport, CoveredMethod, MethodState, Uncovered};

pub struct CoverageText;

impl CoverageText {
    const SHOWN_MISSING: usize = 20;

    #[must_use]
    pub fn summary(report: &CoverageReport) -> String {
        format!(
            "coverage: {} of {} public methods on annotated classes ran in a test case.",
            report.covered(),
            report.total()
        )
    }

    #[must_use]
    pub fn missing(report: &CoverageReport) -> Vec<String> {
        let uncovered = report.uncovered();
        let mut lines: Vec<String> = uncovered
            .iter()
            .take(Self::SHOWN_MISSING)
            .map(Self::missing_line)
            .collect();
        lines.extend(Self::more_line(uncovered.len()));
        lines
    }

    #[must_use]
    pub fn details(report: &CoverageReport) -> Vec<String> {
        report.classes.iter().flat_map(Self::class_lines).collect()
    }

    fn missing_line(entry: &Uncovered) -> String {
        if entry.recorded {
            return format!(
                "  no test case called {}.{} ({})",
                entry.type_name, entry.method, entry.file
            );
        }
        format!(
            "  no test names {}.{} ({}) [name check]",
            entry.type_name, entry.method, entry.file
        )
    }

    fn more_line(count: usize) -> Option<String> {
        let hidden = count.saturating_sub(Self::SHOWN_MISSING);
        (hidden > 0).then(|| format!("  … and {hidden} more"))
    }

    fn class_lines(class: &ClassCoverage) -> Vec<String> {
        let heading = format!(
            "  {} ({}){}",
            class.type_name,
            class.file,
            Self::check_label(class)
        );
        let mut lines = vec![heading];
        lines.extend(class.methods.iter().map(Self::method_line));
        lines
    }

    fn check_label(class: &ClassCoverage) -> String {
        if class.recorded {
            return String::new();
        }
        format!(
            " [name check: no .{} test unit could run here, so ty checks names]",
            class.language
        )
    }

    fn method_line(method: &CoveredMethod) -> String {
        match &method.state {
            MethodState::Called(case) => format!("    called   {} (first by {case})", method.name),
            MethodState::Named => format!("    named    {}", method.name),
            MethodState::Missing => format!("    missing  {}", method.name),
        }
    }
}
