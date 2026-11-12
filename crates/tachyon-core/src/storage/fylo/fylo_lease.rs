use serde_json::Value;

use crate::Failure;
use crate::storage::failure;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FyloLease {
    Server,
    Call,
}

impl FyloLease {
    const KEY: &'static str = "lease";
    const SERVER: &'static str = "server";
    const CALL: &'static str = "call";

    pub(super) fn from_settings(settings: &Value) -> Result<Self, Failure> {
        let name = settings
            .get(Self::KEY)
            .map_or(Some(Self::SERVER), Value::as_str);
        Self::named(name)
    }

    fn named(name: Option<&str>) -> Result<Self, Failure> {
        match name {
            Some(Self::SERVER) => Ok(Self::Server),
            Some(Self::CALL) => Ok(Self::Call),
            _ => Err(failure(
                "The FYLO datasource lease is \"server\" (the default) or \"call\".",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::FyloLease;

    #[test]
    fn a_datasource_without_a_lease_keeps_one_fylo_process_for_the_server() {
        let lease = FyloLease::from_settings(&json!({"driver": "fylo"})).ok();
        assert_eq!(lease, Some(FyloLease::Server));
    }

    #[test]
    fn the_server_lease_can_be_named() {
        let lease = FyloLease::from_settings(&json!({"lease": "server"})).ok();
        assert_eq!(lease, Some(FyloLease::Server));
    }

    #[test]
    fn the_call_lease_is_chosen_by_name() {
        let lease = FyloLease::from_settings(&json!({"lease": "call"})).ok();
        assert_eq!(lease, Some(FyloLease::Call));
    }

    #[test]
    fn an_unknown_lease_is_refused() {
        let refused = FyloLease::from_settings(&json!({"lease": "request"}));
        let message = refused.err().map(|failure| failure.to_string());
        assert!(message.is_some_and(|message| message.contains("\"call\"")));
    }

    #[test]
    fn a_lease_that_is_not_text_is_refused() {
        assert!(FyloLease::from_settings(&json!({"lease": true})).is_err());
    }
}
