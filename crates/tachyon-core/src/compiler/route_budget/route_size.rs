use super::BudgetLimit;
use crate::failure::diagnostic;
use tachyon_diagnostics::Diagnostic;

pub(super) struct RouteSize {
    route: String,
    limit: BudgetLimit,
    files: Vec<(String, u64)>,
}

impl RouteSize {
    const CODE: u16 = 1337;
    const LISTED_FILES: usize = 5;
    const BYTES_PER_KILOBYTE: u64 = 1024;

    pub(super) fn new(route: String, limit: BudgetLimit, files: Vec<(String, u64)>) -> Self {
        Self {
            route,
            limit,
            files,
        }
    }

    pub(super) fn is_over(&self) -> bool {
        self.total() > self.limit.bytes
    }

    pub(super) fn diagnostic(&self) -> Diagnostic {
        let message = format!(
            "Route '{}' loads {} KB of {}, and its budget is {} KB.",
            self.route,
            Self::kilobytes(self.total()),
            self.limit.name,
            Self::kilobytes(self.limit.bytes)
        );
        let help = format!(
            "The biggest files: {}. Load less on this route: split a big service, remove a library that the route does not need, or move work to the server. If the size is right, raise tac.budgets.{} in client/manifest.json. Sizes are the bytes that Tachyon sends: the Brotli copy when the build wrote one, else the file itself (ADR 0074).",
            self.biggest_files(),
            self.limit.setting
        );
        diagnostic(Self::CODE, message, Some(help), None)
    }

    fn total(&self) -> u64 {
        self.files.iter().map(|(_, size)| size).sum()
    }

    fn biggest_files(&self) -> String {
        let mut sorted = self.files.clone();
        sorted.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
        let listed: Vec<String> = sorted
            .iter()
            .take(Self::LISTED_FILES)
            .map(|(path, size)| format!("/{path} ({} KB)", Self::kilobytes(*size)))
            .collect();
        listed.join(", ")
    }

    fn kilobytes(bytes: u64) -> u64 {
        bytes.div_ceil(Self::BYTES_PER_KILOBYTE)
    }
}
