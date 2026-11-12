use std::ops::RangeInclusive;
use std::time::Duration;

use serde_json::Value;

use crate::Failure;
use crate::storage::failure;

pub(super) struct OperationTimeout;

impl OperationTimeout {
    const KEY: &'static str = "operationTimeoutMs";
    const DEFAULT_MILLISECONDS: u64 = 5_000;
    const ALLOWED_MILLISECONDS: RangeInclusive<u64> = 1_000..=60_000;

    pub(super) fn from_settings(settings: &Value) -> Result<Duration, Failure> {
        let named = settings
            .get(Self::KEY)
            .map_or(Some(Self::DEFAULT_MILLISECONDS), Value::as_u64);
        let milliseconds = Self::allowed(named)?;
        Ok(Duration::from_millis(milliseconds))
    }

    fn allowed(milliseconds: Option<u64>) -> Result<u64, Failure> {
        milliseconds
            .filter(|milliseconds| Self::ALLOWED_MILLISECONDS.contains(milliseconds))
            .ok_or_else(|| failure("FYLO operationTimeoutMs is a whole number from 1000 to 60000."))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::OperationTimeout;

    #[test]
    fn a_datasource_without_the_setting_keeps_five_seconds() {
        let timeout = OperationTimeout::from_settings(&json!({"driver": "fylo"})).ok();
        assert_eq!(timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn a_deployment_can_give_store_calls_more_time() {
        let timeout = OperationTimeout::from_settings(&json!({"operationTimeoutMs": 20_000})).ok();
        assert_eq!(timeout, Some(Duration::from_secs(20)));
    }

    #[test]
    fn the_bounds_are_one_second_and_one_minute() {
        for allowed in [1_000, 60_000] {
            let settings = json!({ "operationTimeoutMs": allowed });
            assert!(OperationTimeout::from_settings(&settings).is_ok());
        }
        for refused in [
            json!(999),
            json!(60_001),
            json!(-5),
            json!(2.5),
            json!("5000"),
        ] {
            let settings = json!({ "operationTimeoutMs": refused });
            assert!(
                OperationTimeout::from_settings(&settings).is_err(),
                "{settings}"
            );
        }
    }
}
