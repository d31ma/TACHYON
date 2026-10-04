use super::MigrationFinding;
use super::built_role::BuiltRole;
use std::path::Path;

pub(super) struct BuiltClient;

impl BuiltClient {
    const FEATURE: &'static str = "service.builds_client";
    const SERVICES: &'static str = "client/services/";
    const SUFFIX: &'static str = "Client";
    const CALLS_AFTER: [&'static str; 4] = ["::new(", "::with_context(", ".new", "()"];
    const CALLS_BEFORE: [&'static str; 1] = ["new "];
    const DETAIL: &'static str = "A service builds its HTTPClient itself. Tachyon gives a service its client through Needs, so the client shares the service's lifetime and a test can give the service a Fake.";
    const ROLE: BuiltRole = BuiltRole {
        feature: Self::FEATURE,
        suffix: Self::SUFFIX,
        detail: Self::DETAIL,
        action: Self::ACTION,
    };
    const ACTION: &'static str = "Remove the code that builds the client, such as CatalogClient::new() or new CatalogClient(), and ask for it with Needs: in Rust #[Needs] catalog_client: std::sync::Arc<CatalogClient>, in the other languages a Needs field.";

    pub(super) fn read_finding(root: &Path, relative: &str) -> Option<MigrationFinding> {
        relative.starts_with(Self::SERVICES).then_some(())?;
        Self::finding(relative, &BuiltRole::source(root, relative)?)
    }

    pub(super) fn finding(relative: &str, source: &str) -> Option<MigrationFinding> {
        Self::ROLE.finding(relative, source, Self::builds)
    }

    fn builds(source: &str, at: usize) -> bool {
        let before = &source[..at];
        let name_start = BuiltRole::name_start(before);
        let after = &source[at + Self::SUFFIX.len()..];
        let named = [
            name_start < at,
            !after.starts_with(BuiltRole::is_name_character),
        ] == [true, true];
        let called_after = Self::CALLS_AFTER.iter().any(|call| after.starts_with(call));
        let called_before = Self::CALLS_BEFORE
            .iter()
            .any(|call| before[..name_start].ends_with(call));
        [named, [called_after, called_before].contains(&true)] == [true, true]
    }
}
