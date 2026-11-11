use serde_json::{Map, Value};
use std::ops::RangeInclusive;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReconnectPolicy {
    max_tries: u64,
    delay_ms: u64,
    max_delay_ms: u64,
    jitter_ms: u64,
}

impl ReconnectPolicy {
    pub(crate) const PREFIX: &'static str = "/.tachyon/reconnect/";
    const MAX_TRIES: &'static str = "maxTries";
    const DELAY: &'static str = "delayMs";
    const MAX_DELAY: &'static str = "maxDelayMs";
    const JITTER: &'static str = "jitterMs";
    const KEYS: [&'static str; 4] = [Self::MAX_TRIES, Self::DELAY, Self::MAX_DELAY, Self::JITTER];
    const UNLIMITED: u64 = 0;
    const DEFAULT_TRIES: u64 = 5;
    const DEFAULT_DELAY_MS: u64 = 1_000;
    const DEFAULT_MAX_DELAY_MS: u64 = 16_000;
    const UNLIMITED_JITTER_MS: u64 = 1_000;
    const TRIES: RangeInclusive<u64> = 0..=100;
    const DELAYS: RangeInclusive<u64> = 100..=60_000;
    const MAX_DELAYS: RangeInclusive<u64> = 100..=600_000;
    const JITTERS: RangeInclusive<u64> = 0..=60_000;
    const FORM: &'static str = "reconnect takes maxTries (0 means no limit, at most 100), delayMs (100 to 60000), maxDelayMs (100 to 600000, not below delayMs) and jitterMs (0 to 60000, above 0 when maxTries is 0), for example {\"maxTries\": 0, \"delayMs\": 1000, \"maxDelayMs\": 30000, \"jitterMs\": 1000}";

    pub(crate) fn read(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or_else(Self::form)?;
        Self::check_keys(object)?;
        let max_tries = Self::number(object, Self::MAX_TRIES, Self::DEFAULT_TRIES, Self::TRIES)?;
        let unlimited = max_tries == Self::UNLIMITED;
        let jitter_default = [0, Self::UNLIMITED_JITTER_MS][usize::from(unlimited)];
        let policy = Self {
            max_tries,
            delay_ms: Self::number(object, Self::DELAY, Self::DEFAULT_DELAY_MS, Self::DELAYS)?,
            max_delay_ms: Self::number(
                object,
                Self::MAX_DELAY,
                Self::DEFAULT_MAX_DELAY_MS,
                Self::MAX_DELAYS,
            )?,
            jitter_ms: Self::number(object, Self::JITTER, jitter_default, Self::JITTERS)?,
        };
        policy.check_delays()?;
        policy.check_jitter()?;
        Ok(policy)
    }

    pub(crate) fn addressed(&self, path: &str) -> String {
        format!(
            "{}{}/{}/{}/{}{path}",
            Self::PREFIX,
            self.max_tries,
            self.delay_ms,
            self.max_delay_ms,
            self.jitter_ms
        )
    }

    fn form() -> String {
        String::from(Self::FORM)
    }

    fn check_keys(object: &Map<String, Value>) -> Result<(), String> {
        let known = object.keys().all(|key| Self::KEYS.contains(&key.as_str()));
        known.then_some(()).ok_or_else(Self::form)
    }

    fn number(
        object: &Map<String, Value>,
        key: &str,
        default: u64,
        allowed: RangeInclusive<u64>,
    ) -> Result<u64, String> {
        let written = object.get(key).map_or(Some(default), Value::as_u64);
        written
            .filter(|number| allowed.contains(number))
            .ok_or_else(Self::form)
    }

    fn check_delays(&self) -> Result<(), String> {
        (self.delay_ms <= self.max_delay_ms)
            .then_some(())
            .ok_or_else(Self::form)
    }

    fn check_jitter(&self) -> Result<(), String> {
        let unlimited_without_jitter = [self.max_tries, self.jitter_ms] == [Self::UNLIMITED, 0];
        (!unlimited_without_jitter)
            .then_some(())
            .ok_or_else(Self::form)
    }
}
