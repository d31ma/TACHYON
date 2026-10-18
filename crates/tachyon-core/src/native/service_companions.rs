use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;

pub(super) struct ServiceCompanions;

impl ServiceCompanions {
    const NUMBERS: [(NativeCompanion, u8); 2] =
        [(NativeCompanion::Rust, 1), (NativeCompanion::CSharp, 2)];

    pub(super) fn number(language: NativeCompanion) -> u8 {
        Self::NUMBERS
            .iter()
            .find(|(known, _)| *known == language)
            .map_or(0, |(_, number)| *number)
    }

    pub(super) fn c_table(companions: &[NativeCompanionInput]) -> String {
        let rows: String = companions
            .iter()
            .filter(|item| Self::links_natively(item))
            .map(Self::c_row)
            .collect();
        include_str!("service_companions.h").replace("__SERVICE_COMPANIONS__", &rows)
    }

    fn links_natively(item: &NativeCompanionInput) -> bool {
        Self::number(item.language) != 0 && !item.web_assembly && !item.application_provider
    }

    fn c_row(item: &NativeCompanionInput) -> String {
        let service = super::host::c_string_escape(&item.service);
        format!("  {{\"{service}\", {}}},\n", Self::number(item.language))
    }
}
