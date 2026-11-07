use super::ValueSource;
use super::testing::Unit;
use serde_json::Value;

pub(crate) struct UnitTable {
    pub(crate) name: String,
    pub(crate) configuration: String,
    pub(crate) partition: String,
    pub(crate) profiles: Value,
    pub(crate) properties: Value,
    pub(crate) cases: Value,
    pub(crate) fakes: Vec<String>,
}

impl UnitTable {
    pub(super) fn of(unit: &Unit) -> Self {
        Self {
            name: unit.name.clone(),
            configuration: unit.configuration.clone(),
            partition: unit.partition.clone(),
            profiles: unit.profiles.clone(),
            properties: unit.properties.clone(),
            cases: ValueSource::case_table(&unit.cases, &unit.inputs),
            fakes: unit.mocks.clone(),
        }
    }
}
