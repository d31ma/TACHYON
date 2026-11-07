use super::{EnvironmentPolicy, HandlerRuntimePrograms};
use crate::Failure;
use crate::failure::diagnostic;
use crate::handler::YonIsolationPolicy;
use std::time::Duration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerSupervisorOptions {
    pub application: crate::ApplicationConfig,
    pub default_timeout: Duration,
    pub cancellation_grace: Duration,
    pub stderr_limit: usize,
    pub max_concurrency: usize,
    pub runtimes: HandlerRuntimePrograms,
    pub environment: EnvironmentPolicy,
    pub isolation: YonIsolationPolicy,
}

impl Default for HandlerSupervisorOptions {
    fn default() -> Self {
        Self {
            application: crate::ApplicationConfig::default(),
            default_timeout: Self::DEFAULT_TIMEOUT,
            cancellation_grace: Self::DEFAULT_CANCEL_GRACE,
            stderr_limit: Self::DEFAULT_STDERR_LIMIT,
            max_concurrency: Self::DEFAULT_MAX_CONCURRENCY,
            runtimes: HandlerRuntimePrograms::default(),
            environment: EnvironmentPolicy::default(),
            isolation: YonIsolationPolicy::default(),
        }
    }
}

impl HandlerSupervisorOptions {
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
    const MAX_TIMEOUT: Duration = Duration::from_mins(5);
    const DEFAULT_CANCEL_GRACE: Duration = Duration::from_millis(100);
    const MAX_CANCEL_GRACE: Duration = Duration::from_secs(5);
    const DEFAULT_STDERR_LIMIT: usize = 64 * 1024;
    const DEFAULT_MAX_CONCURRENCY: usize = 16;

    pub fn from_environment() -> Result<Self, Failure> {
        Ok(Self {
            isolation: YonIsolationPolicy::from_environment()?,
            ..Self::default()
        })
    }

    pub(super) fn check_limits(&self) -> Result<(), Failure> {
        self.check_positive_limits()?;
        self.check_timeout()?;
        self.check_cancellation_grace()
    }

    fn check_positive_limits(&self) -> Result<(), Failure> {
        if self.max_concurrency == 0 || self.stderr_limit == 0 {
            return Err(Self::invalid_limits());
        }
        Ok(())
    }

    fn check_timeout(&self) -> Result<(), Failure> {
        if self.default_timeout.is_zero() || self.default_timeout > Self::MAX_TIMEOUT {
            return Err(Self::invalid_limits());
        }
        Ok(())
    }

    fn check_cancellation_grace(&self) -> Result<(), Failure> {
        if self.cancellation_grace > Self::MAX_CANCEL_GRACE {
            return Err(Self::invalid_limits());
        }
        Ok(())
    }

    fn invalid_limits() -> Failure {
        Failure::one(diagnostic(
            2007,
            "Handler supervisor resource limits are invalid.",
            Some(String::from(
                "Use 1..=300 seconds, positive concurrency/stderr limits, and at most 5 seconds cancellation grace.",
            )),
            None,
        ))
    }
}
