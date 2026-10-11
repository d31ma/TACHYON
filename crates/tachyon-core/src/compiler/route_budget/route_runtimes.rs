use super::WebRuntime;
use std::collections::BTreeSet;

pub(super) struct RouteRuntimes {
    reached: Vec<WebRuntime>,
}

impl RouteRuntimes {
    const BYTES_PER_KILOBYTE: u64 = 1024;
    const DECISION: &'static str = "ADR 0100";

    pub(super) fn of(loaded: &BTreeSet<String>) -> Self {
        let mut reached = WebRuntime::reached(loaded);
        reached.sort_by_key(|runtime| std::cmp::Reverse(runtime.kilobytes));
        Self { reached }
    }

    pub(super) fn allowance_bytes(&self) -> u64 {
        let kilobytes: u64 = self.counted().iter().map(|runtime| runtime.kilobytes).sum();
        kilobytes.saturating_mul(Self::BYTES_PER_KILOBYTE)
    }

    pub(super) fn basis(&self, base_kilobytes: u64) -> String {
        let largest = self.reached.first().map_or("", |runtime| runtime.name);
        let added: Vec<String> = self
            .counted()
            .iter()
            .map(|runtime| format!("{} KB for {}", runtime.kilobytes, runtime.name))
            .collect();
        format!(
            " It is {base_kilobytes} KB for the page and its largest web runtime, {largest}, plus {} ({}).",
            added.join(", "),
            Self::DECISION
        )
    }

    fn counted(&self) -> &[WebRuntime] {
        self.reached.get(1..).unwrap_or_default()
    }
}
