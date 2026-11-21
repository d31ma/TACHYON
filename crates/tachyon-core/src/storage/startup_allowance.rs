use std::ops::RangeInclusive;
use std::time::Duration;

use serde_json::Value;

use super::failure;
use crate::Failure;

pub(crate) struct StartupAllowance;

impl StartupAllowance {
    const KEY: &'static str = "startup";
    const DEFAULT_MILLISECONDS: u64 = 20_000;
    const ALLOWED_MILLISECONDS: RangeInclusive<u64> = 1_000..=120_000;

    pub(super) fn from_configuration(yon: &Value) -> Result<Duration, Failure> {
        let named = yon
            .pointer("/datasource/startup")
            .map_or(Some(Self::DEFAULT_MILLISECONDS), Value::as_u64);
        named
            .filter(|milliseconds| Self::ALLOWED_MILLISECONDS.contains(milliseconds))
            .map(Duration::from_millis)
            .ok_or_else(|| {
                failure("The datasource startup setting is a whole number of milliseconds from 1000 to 120000.")
            })
    }

    pub(super) fn without_its_setting(datasource: Option<Value>) -> Option<Value> {
        let mut datasource = datasource;
        datasource
            .iter_mut()
            .filter_map(Value::as_object_mut)
            .for_each(|settings| {
                settings.remove(Self::KEY);
            });
        datasource
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::StartupAllowance;

    #[test]
    fn the_allowance_is_twenty_seconds_unless_set() {
        let default =
            StartupAllowance::from_configuration(&json!({"datasource": {"driver": "fylo"}}));
        assert_eq!(default.ok(), Some(Duration::from_secs(20)));
        let set = StartupAllowance::from_configuration(&json!({"datasource": {"startup": 45_000}}));
        assert_eq!(set.ok(), Some(Duration::from_secs(45)));
    }

    #[test]
    fn the_bounds_are_one_second_and_two_minutes() {
        for refused in [json!(999), json!(120_001), json!("20000"), json!(-1)] {
            let settings = json!({"datasource": {"startup": refused}});
            assert!(
                StartupAllowance::from_configuration(&settings).is_err(),
                "{settings}"
            );
        }
    }

    #[test]
    fn the_drivers_never_see_the_setting() {
        let datasource = json!({"driver": "fylo", "startup": 30_000});
        let stripped = StartupAllowance::without_its_setting(Some(datasource));
        assert_eq!(stripped, Some(json!({"driver": "fylo"})));
    }
}
