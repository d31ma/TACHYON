use super::YonIsolationPolicy;
use super::environment_names::EnvironmentNames;
use crate::Failure;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirecrackerIsolation {
    pub(super) driver: PathBuf,
    pub(super) pool: String,
    pub(super) vcpus: u8,
    pub(super) memory_mib: u32,
}

impl FirecrackerIsolation {
    const DEFAULT_POOL: &str = "default";
    const DEFAULT_VCPUS: u8 = 1;
    const DEFAULT_MEMORY_MIB: u32 = 256;
    const MIN_MEMORY_MIB: u32 = 128;
    const MAX_MEMORY_MIB: u32 = 32_768;
    const MAX_VCPUS: u8 = 32;
    const MAX_POOL_BYTES: usize = 64;
    const POOL_SYMBOLS: &'static [u8] = b"_-";

    pub(super) fn from_lookup<F>(lookup: &F) -> Result<Self, Failure>
    where
        F: Fn(&str) -> Option<OsString>,
    {
        let driver = Self::required_unicode(lookup, EnvironmentNames::DRIVER).map(PathBuf::from)?;
        Self::validate_driver(&driver)?;
        let pool = Self::pool_from_lookup(lookup)?;
        let vcpus = Self::parse_number(
            YonIsolationPolicy::optional_unicode(lookup, EnvironmentNames::VCPUS)?,
            EnvironmentNames::VCPUS,
            Self::DEFAULT_VCPUS,
            1,
            Self::MAX_VCPUS,
        )?;
        let memory_mib = Self::parse_number(
            YonIsolationPolicy::optional_unicode(lookup, EnvironmentNames::MEMORY)?,
            EnvironmentNames::MEMORY,
            Self::DEFAULT_MEMORY_MIB,
            Self::MIN_MEMORY_MIB,
            Self::MAX_MEMORY_MIB,
        )?;
        Self::require_denied_egress(lookup)?;
        Ok(Self {
            driver,
            pool,
            vcpus,
            memory_mib,
        })
    }

    pub(crate) fn driver(&self) -> &Path {
        &self.driver
    }

    pub(crate) fn append_arguments(&self, command: &mut tokio::process::Command) {
        command
            .arg("invoke")
            .arg("--protocol")
            .arg("handler-v1")
            .arg("--pool")
            .arg(&self.pool)
            .arg("--vcpus")
            .arg(self.vcpus.to_string())
            .arg("--memory-mib")
            .arg(self.memory_mib.to_string())
            .arg("--egress")
            .arg("deny");
    }

    fn pool_from_lookup<F>(lookup: &F) -> Result<String, Failure>
    where
        F: Fn(&str) -> Option<OsString>,
    {
        let pool = YonIsolationPolicy::optional_unicode(lookup, EnvironmentNames::POOL)?
            .unwrap_or_else(|| String::from(Self::DEFAULT_POOL));
        Self::check_pool(&pool)?;
        Ok(pool)
    }

    fn check_pool(pool: &str) -> Result<(), Failure> {
        if pool.is_empty()
            || pool.len() > Self::MAX_POOL_BYTES
            || !pool.bytes().all(Self::pool_byte)
        {
            return Err(YonIsolationPolicy::configuration_failure(format!(
                "{} must contain 1 to 64 ASCII letters, digits, underscores, or hyphens.",
                EnvironmentNames::POOL
            )));
        }
        Ok(())
    }

    fn pool_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || Self::POOL_SYMBOLS.contains(&byte)
    }

    fn require_denied_egress<F>(lookup: &F) -> Result<(), Failure>
    where
        F: Fn(&str) -> Option<OsString>,
    {
        let egress = YonIsolationPolicy::optional_unicode(lookup, EnvironmentNames::EGRESS)?
            .unwrap_or_else(|| String::from("deny"));
        if egress != "deny" {
            return Err(YonIsolationPolicy::configuration_failure(format!(
                "{} currently supports only 'deny'.",
                EnvironmentNames::EGRESS
            )));
        }
        Ok(())
    }

    fn required_unicode<F>(lookup: &F, name: &str) -> Result<String, Failure>
    where
        F: Fn(&str) -> Option<OsString>,
    {
        YonIsolationPolicy::optional_unicode(lookup, name)?.ok_or_else(|| {
            YonIsolationPolicy::configuration_failure(format!(
                "{name} is required when {}=firecracker.",
                EnvironmentNames::ISOLATION
            ))
        })
    }

    fn parse_number<T>(
        value: Option<String>,
        name: &str,
        default: T,
        minimum: T,
        maximum: T,
    ) -> Result<T, Failure>
    where
        T: Copy + Ord + std::str::FromStr,
    {
        value.map_or(Ok(default), |value| {
            Self::parse_in_range(&value, name, minimum, maximum)
        })
    }

    fn parse_in_range<T>(value: &str, name: &str, minimum: T, maximum: T) -> Result<T, Failure>
    where
        T: Copy + Ord + std::str::FromStr,
    {
        let parsed = value.parse::<T>().map_err(|_| {
            YonIsolationPolicy::configuration_failure(format!("{name} must be a base-10 integer."))
        })?;
        if parsed < minimum || parsed > maximum {
            return Err(YonIsolationPolicy::configuration_failure(format!(
                "{name} is outside its supported range."
            )));
        }
        Ok(parsed)
    }

    fn validate_driver(driver: &Path) -> Result<(), Failure> {
        let driver_name = EnvironmentNames::DRIVER;
        Self::check_absolute_driver(driver)?;
        let metadata = fs::symlink_metadata(driver).map_err(|error| {
            YonIsolationPolicy::configuration_failure(format!(
                "Cannot inspect {driver_name}: {error}"
            ))
        })?;
        Self::check_regular_driver(&metadata)?;
        #[cfg(unix)]
        Self::check_driver_is_executable(&metadata)?;
        #[cfg(unix)]
        Self::check_driver_is_not_shared_writable(&metadata)?;
        Ok(())
    }

    fn check_absolute_driver(driver: &Path) -> Result<(), Failure> {
        if !driver.is_absolute() {
            return Err(YonIsolationPolicy::configuration_failure(format!(
                "{} must be an absolute path.",
                EnvironmentNames::DRIVER
            )));
        }
        Ok(())
    }

    fn check_regular_driver(metadata: &fs::Metadata) -> Result<(), Failure> {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(YonIsolationPolicy::configuration_failure(format!(
                "{} must name a regular, non-symlinked file.",
                EnvironmentNames::DRIVER
            )));
        }
        Ok(())
    }

    #[cfg(unix)]
    fn check_driver_is_executable(metadata: &fs::Metadata) -> Result<(), Failure> {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(YonIsolationPolicy::configuration_failure(format!(
                "{} is not executable.",
                EnvironmentNames::DRIVER
            )));
        }
        Ok(())
    }

    #[cfg(unix)]
    fn check_driver_is_not_shared_writable(metadata: &fs::Metadata) -> Result<(), Failure> {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o022 != 0 {
            return Err(YonIsolationPolicy::configuration_failure(format!(
                "{} must not be writable by its group or other users.",
                EnvironmentNames::DRIVER
            )));
        }
        Ok(())
    }
}
