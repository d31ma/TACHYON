use super::YonIsolationPolicy;
use super::environment_names::EnvironmentNames;
use crate::Failure;
use std::ffi::OsString;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WarmSettings {
    pub(crate) max_requests: u32,
    pub(crate) idle_limit: Duration,
}

impl Default for WarmSettings {
    fn default() -> Self {
        Self::new(
            Self::DEFAULT_MAX_REQUESTS,
            Duration::from_secs(Self::DEFAULT_IDLE_SECONDS),
        )
    }
}

impl WarmSettings {
    const DEFAULT_MAX_REQUESTS: u32 = 1_000;
    const LARGEST_MAX_REQUESTS: u32 = 1_000_000;
    const DEFAULT_IDLE_SECONDS: u64 = 60;
    const LARGEST_IDLE_SECONDS: u64 = 3_600;

    #[must_use]
    pub const fn new(max_requests: u32, idle_limit: Duration) -> Self {
        Self {
            max_requests,
            idle_limit,
        }
    }

    pub(super) fn from_lookup<F>(lookup: &F) -> Result<Self, Failure>
    where
        F: Fn(&str) -> Option<OsString>,
    {
        let max_requests = YonIsolationPolicy::parse_number(
            YonIsolationPolicy::optional_unicode(lookup, EnvironmentNames::WARM_MAX_REQUESTS)?,
            EnvironmentNames::WARM_MAX_REQUESTS,
            Self::DEFAULT_MAX_REQUESTS,
            1,
            Self::LARGEST_MAX_REQUESTS,
        )?;
        let idle_seconds = YonIsolationPolicy::parse_number(
            YonIsolationPolicy::optional_unicode(lookup, EnvironmentNames::WARM_IDLE_SECONDS)?,
            EnvironmentNames::WARM_IDLE_SECONDS,
            Self::DEFAULT_IDLE_SECONDS,
            1,
            Self::LARGEST_IDLE_SECONDS,
        )?;
        Ok(Self::new(max_requests, Duration::from_secs(idle_seconds)))
    }
}
