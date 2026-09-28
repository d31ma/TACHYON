use super::finding::Finding;
use super::method::Method;

pub(super) struct Scanned {
    pub(super) method: Method,
    pub(super) findings: Vec<Finding>,
}

impl Scanned {
    pub(super) fn new(method: Method, findings: Vec<Finding>) -> Self {
        Self { method, findings }
    }

    pub(super) fn decisions(&self) -> Vec<&Finding> {
        self.findings.iter().filter(|found| found.counts).collect()
    }

    pub(super) fn branches(&self) -> bool {
        self.findings.iter().any(|found| found.branches)
    }

    pub(super) fn is_branching_helper(&self) -> bool {
        !self.method.public & self.branches()
    }

    pub(super) fn words(&self) -> String {
        let mut words: Vec<&str> = self
            .findings
            .iter()
            .map(|found| found.word.as_str())
            .collect();
        words.sort_unstable();
        words.dedup();
        words.join("`, `")
    }
}
