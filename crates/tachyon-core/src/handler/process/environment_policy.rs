use crate::Failure;
use crate::failure::diagnostic;
use std::collections::BTreeSet;
use std::ffi::OsString;
use tokio::process::Command;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnvironmentPolicy {
    allowed: BTreeSet<String>,
}

impl EnvironmentPolicy {
    #[cfg(windows)]
    const BASELINE_NAMES: &'static [&'static str] = &[
        "PATH",
        "PATHEXT",
        "SYSTEMROOT",
        "WINDIR",
        "COMSPEC",
        "TEMP",
        "TMP",
    ];
    #[cfg(not(windows))]
    const BASELINE_NAMES: &'static [&'static str] = &["PATH", "LANG", "LC_ALL", "TMPDIR"];
    const PRIVATE_NAMES: [&'static str; 3] = [
        "YON_APPLICATION_JSON",
        "YON_HANDLER_PROTOCOL",
        "YON_SOCKET_FRAMES",
    ];
    const PRIVATE_PREFIX: &'static str = "YON_RESOURCE_";

    pub fn from_names<I, S>(names: I) -> Result<Self, Failure>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut policy = Self::default();
        names.into_iter().try_for_each(|name| policy.allow(name))?;
        Ok(policy)
    }

    pub fn allow(&mut self, name: impl Into<String>) -> Result<(), Failure> {
        let name = name.into();
        Self::check_name(&name)?;
        self.allowed.insert(name);
        Ok(())
    }

    pub(super) fn apply(&self, command: &mut Command) {
        self.apply_std(command.as_std_mut());
    }

    pub(crate) fn apply_std(&self, command: &mut std::process::Command) {
        self.apply_external(command);
        command.env("YON_HANDLER_PROTOCOL", "1");
    }

    pub(crate) fn apply_external(&self, command: &mut std::process::Command) {
        command.env_clear();
        command.envs(self.present_variables());
    }

    fn present_variables(&self) -> impl Iterator<Item = (&str, OsString)> {
        self.environment_names()
            .filter_map(|name| std::env::var_os(name).map(|value| (name, value)))
    }

    fn environment_names(&self) -> impl Iterator<Item = &str> {
        Self::BASELINE_NAMES
            .iter()
            .copied()
            .chain(self.allowed.iter().map(String::as_str))
            .filter(|name| !Self::is_private_environment_name(name))
    }

    fn is_private_environment_name(name: &str) -> bool {
        let name = name.to_ascii_uppercase();
        Self::PRIVATE_NAMES.contains(&name.as_str()) || name.starts_with(Self::PRIVATE_PREFIX)
    }

    fn check_name(name: &str) -> Result<(), Failure> {
        let mut bytes = name.bytes();
        if !bytes.next().is_some_and(Self::starts_name) || !bytes.all(Self::continues_name) {
            return Err(Failure::one(diagnostic(
                2006,
                format!("Environment allowlist name '{name}' is invalid."),
                Some(String::from(
                    "Use ASCII letters, digits, and underscores, beginning with a letter or underscore.",
                )),
                None,
            )));
        }
        Ok(())
    }

    fn starts_name(byte: u8) -> bool {
        byte.is_ascii_alphabetic() || byte == b'_'
    }

    fn continues_name(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::EnvironmentPolicy;

    #[test]
    fn external_allowlists_cannot_include_private_protocol_values() {
        let reserved = [
            "YON_APPLICATION_JSON",
            "yon_application_json",
            "YON_RESOURCE_TOKEN",
            "Yon_Resource_Future",
            "YON_HANDLER_PROTOCOL",
            "yon_handler_protocol",
            "YON_SOCKET_FRAMES",
        ];
        let policy =
            EnvironmentPolicy::from_names(reserved.into_iter().chain(["YON_PUBLIC_SETTING"]))
                .expect("allowlist");
        let names: Vec<_> = policy.environment_names().collect();
        assert!(names.contains(&"YON_PUBLIC_SETTING"));
        for name in reserved {
            assert!(!names.contains(&name), "private name {name}");
        }
    }
}
