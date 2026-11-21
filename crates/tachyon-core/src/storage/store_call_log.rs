use std::ops::RangeInclusive;
use std::time::Duration;

use serde_json::Value;

use super::failure;
use crate::Failure;

pub(crate) struct StoreCallLog {
    slow_after: Duration,
    every_call: bool,
    datasource: &'static str,
}

impl StoreCallLog {
    const SLOW_KEY: &'static str = "slowcall";
    const EVERY_CALL_KEY: &'static str = "logcalls";
    const DEFAULT_SLOW_MILLISECONDS: u64 = 1_000;
    const ALLOWED_SLOW_MILLISECONDS: RangeInclusive<u64> = 1..=60_000;
    const DRIVERS: [&'static str; 6] = [
        "postgresql",
        "mysql",
        "mariadb",
        "sqlserver",
        "sqlite",
        "fylo",
    ];
    const NO_DATASOURCE: &'static str = "none";
    const REDIS: &'static str = "redis";

    pub(crate) fn from_configuration(yon: &Value) -> Result<Self, Failure> {
        let settings = yon.get("datasource").unwrap_or(&Value::Null);
        Ok(Self {
            slow_after: Self::slow_after(settings)?,
            every_call: Self::every_call(settings)?,
            datasource: Self::driver(settings),
        })
    }

    pub(super) fn without_its_settings(datasource: Option<Value>) -> Option<Value> {
        let mut datasource = datasource;
        datasource
            .iter_mut()
            .filter_map(Value::as_object_mut)
            .for_each(Self::remove_settings);
        datasource
    }

    pub(crate) fn is_slow(&self, took: Duration) -> bool {
        took >= self.slow_after
    }

    pub(crate) fn logs_every_call(&self) -> bool {
        self.every_call
    }

    pub(crate) fn datasource_of(&self, command: &Value) -> &'static str {
        let redis = command["operation"] == Self::REDIS;
        [self.datasource, Self::REDIS][usize::from(redis)]
    }

    fn remove_settings(settings: &mut serde_json::Map<String, Value>) {
        settings.remove(Self::SLOW_KEY);
        settings.remove(Self::EVERY_CALL_KEY);
    }

    fn slow_after(settings: &Value) -> Result<Duration, Failure> {
        let named = settings
            .get(Self::SLOW_KEY)
            .map_or(Some(Self::DEFAULT_SLOW_MILLISECONDS), Value::as_u64);
        named
            .filter(|milliseconds| Self::ALLOWED_SLOW_MILLISECONDS.contains(milliseconds))
            .map(Duration::from_millis)
            .ok_or_else(|| {
                failure("The datasource slowcall setting is a whole number of milliseconds from 1 to 60000.")
            })
    }

    fn every_call(settings: &Value) -> Result<bool, Failure> {
        settings
            .get(Self::EVERY_CALL_KEY)
            .map_or(Some(false), Value::as_bool)
            .ok_or_else(|| failure("The datasource logcalls setting is true or false."))
    }

    fn driver(settings: &Value) -> &'static str {
        let named = settings["driver"].as_str();
        Self::DRIVERS
            .into_iter()
            .find(|driver| Some(*driver) == named)
            .unwrap_or(Self::NO_DATASOURCE)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::{Value, json};

    use super::StoreCallLog;

    fn log(datasource: &Value) -> StoreCallLog {
        StoreCallLog::from_configuration(&json!({ "datasource": datasource }))
            .unwrap_or_else(|error| panic!("{error}"))
    }

    #[test]
    fn calls_of_one_second_or_more_are_slow_by_default() {
        let log = log(&json!({"driver": "fylo"}));
        assert!(!log.is_slow(Duration::from_millis(999)));
        assert!(log.is_slow(Duration::from_secs(1)));
        assert!(!log.logs_every_call());
    }

    #[test]
    fn the_threshold_and_every_call_logging_are_settings() {
        let log = log(&json!({"driver": "sqlite", "slowcall": 50, "logcalls": true}));
        assert!(log.is_slow(Duration::from_millis(50)));
        assert!(!log.is_slow(Duration::from_millis(49)));
        assert!(log.logs_every_call());
    }

    #[test]
    fn bad_settings_are_refused() {
        for settings in [
            json!({"slowcall": 0}),
            json!({"slowcall": 60_001}),
            json!({"slowcall": "500"}),
            json!({"slowcall": 1.5}),
            json!({"logcalls": "true"}),
        ] {
            let refused = StoreCallLog::from_configuration(&json!({ "datasource": settings }));
            assert!(refused.is_err(), "{settings}");
        }
    }

    #[test]
    fn the_datasource_is_named_by_its_known_driver_or_redis() {
        let log = log(&json!({"driver": "fylo"}));
        assert_eq!(log.datasource_of(&json!({"operation": "save"})), "fylo");
        assert_eq!(log.datasource_of(&json!({"operation": "redis"})), "redis");
        let unknown = log_without_datasource();
        assert_eq!(unknown.datasource_of(&json!({"operation": "sql"})), "none");
    }

    fn log_without_datasource() -> StoreCallLog {
        StoreCallLog::from_configuration(&json!({})).unwrap_or_else(|error| panic!("{error}"))
    }

    #[test]
    fn the_drivers_never_see_the_log_settings() {
        let datasource =
            json!({"driver": "fylo", "url": "fylo:///data", "slowcall": 5, "logcalls": true});
        let stripped = StoreCallLog::without_its_settings(Some(datasource));
        assert_eq!(
            stripped,
            Some(json!({"driver": "fylo", "url": "fylo:///data"}))
        );
        assert_eq!(StoreCallLog::without_its_settings(None), None);
    }
}
