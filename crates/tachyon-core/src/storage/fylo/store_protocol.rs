use serde_json::Value;

use crate::Failure;
use crate::storage::failure;

pub(super) struct StoreProtocol;

impl StoreProtocol {
    const MACHINE_PROTOCOL: u64 = 2;
    const STORE_VERSION: u64 = 1;
    const SQL_VERSION: u64 = 1;
    const NEEDED: &'static str = "This FYLO is older than Tachyon needs. Tachyon needs FYLO machine protocol 2 with the Store operations (capabilities store 1 and sql 1), which FYLO releases after 26.45.07 have. Install the FYLO version that scripts/ci/install-fylo.mjs pins.";

    pub(super) fn check(identity: &Value) -> Result<(), Failure> {
        Self::speaks_store(identity)
            .then_some(())
            .ok_or_else(|| failure(Self::NEEDED))
    }

    fn speaks_store(identity: &Value) -> bool {
        let protocol = identity["protocolVersion"].as_u64() == Some(Self::MACHINE_PROTOCOL);
        let capabilities = &identity["capabilities"];
        let store = Self::version_of(capabilities, "store") >= Self::STORE_VERSION;
        let sql = Self::version_of(capabilities, "sql") >= Self::SQL_VERSION;
        [protocol, store, sql] == [true, true, true]
    }

    fn version_of(capabilities: &Value, name: &str) -> u64 {
        capabilities[name].as_u64().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::StoreProtocol;

    #[test]
    fn a_fylo_with_protocol_two_and_the_store_and_sql_capabilities_is_accepted() {
        let identity = json!({"protocolVersion": 2, "capabilities": {"store": 1, "sql": 1}});
        assert!(StoreProtocol::check(&identity).is_ok());
    }

    #[test]
    fn newer_capability_versions_are_accepted() {
        let identity = json!({"protocolVersion": 2, "capabilities": {"store": 2, "sql": 3}});
        assert!(StoreProtocol::check(&identity).is_ok());
    }

    #[test]
    fn the_released_fylo_with_protocol_one_is_refused_with_the_needed_version() {
        let refused = StoreProtocol::check(&json!({"protocolVersion": 1}))
            .err()
            .map(|failure| failure.to_string())
            .unwrap_or_default();
        assert!(refused.contains("machine protocol 2"), "{refused}");
        assert!(refused.contains("after 26.45.07"), "{refused}");
    }

    #[test]
    fn protocol_two_without_the_store_capability_is_refused() {
        let identity = json!({"protocolVersion": 2, "capabilities": {"sql": 1}});
        assert!(StoreProtocol::check(&identity).is_err());
    }

    #[test]
    fn protocol_two_without_the_sql_capability_is_refused() {
        let identity = json!({"protocolVersion": 2, "capabilities": {"store": 1}});
        assert!(StoreProtocol::check(&identity).is_err());
    }

    #[test]
    fn a_later_machine_protocol_is_refused_until_tachyon_learns_it() {
        let identity = json!({"protocolVersion": 3, "capabilities": {"store": 1, "sql": 1}});
        assert!(StoreProtocol::check(&identity).is_err());
    }
}
