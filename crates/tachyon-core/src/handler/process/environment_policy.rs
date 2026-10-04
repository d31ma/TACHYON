use crate::Failure;
use crate::failure::diagnostic;
use crate::safety::EnvironmentAllowlist;
use std::collections::BTreeSet;
use std::ffi::OsString;
use tokio::process::Command;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnvironmentPolicy {
    allowed: BTreeSet<String>,
    prefixes: BTreeSet<String>,
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
        self.allowed.insert(Self::comparable(&name));
        Ok(())
    }

    pub(crate) fn include(&mut self, allowlist: &EnvironmentAllowlist) {
        let names = allowlist.names().iter().map(|name| Self::comparable(name));
        self.allowed.extend(names);
        let prefixes = allowlist.prefixes().iter();
        self.prefixes
            .extend(prefixes.map(|prefix| Self::comparable(prefix)));
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
        command.envs(self.variables_from(std::env::vars_os()));
    }

    fn variables_from(
        &self,
        host: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Vec<(OsString, OsString)> {
        host.into_iter()
            .filter(|(name, _)| name.to_str().is_some_and(|name| self.passes(name)))
            .collect()
    }

    fn passes(&self, name: &str) -> bool {
        let name = Self::comparable(name);
        let is_listed = [
            Self::BASELINE_NAMES.contains(&name.as_str()),
            self.allowed.contains(&name),
            self.has_listed_prefix(&name),
        ]
        .contains(&true);
        [is_listed, Self::is_private_environment_name(&name)] == [true, false]
    }

    fn has_listed_prefix(&self, name: &str) -> bool {
        self.prefixes
            .iter()
            .any(|prefix| name.starts_with(prefix.as_str()))
    }

    fn comparable(name: &str) -> String {
        if cfg!(windows) {
            return name.to_ascii_uppercase();
        }
        name.to_owned()
    }

    fn is_private_environment_name(name: &str) -> bool {
        let name = name.to_ascii_uppercase();
        Self::PRIVATE_NAMES.contains(&name.as_str()) || name.starts_with(Self::PRIVATE_PREFIX)
    }

    fn check_name(name: &str) -> Result<(), Failure> {
        if !EnvironmentAllowlist::is_name(name) {
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
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::EnvironmentPolicy;
    use crate::safety::EnvironmentAllowlist;
    use std::ffi::OsString;

    struct HostEnvironment;

    impl HostEnvironment {
        const VALUE: &'static str = "value-canary";

        fn with(names: &[&str]) -> Vec<(OsString, OsString)> {
            names
                .iter()
                .map(|name| (OsString::from(name), OsString::from(Self::VALUE)))
                .collect()
        }

        fn passed(policy: &EnvironmentPolicy, names: &[&str]) -> Vec<String> {
            policy
                .variables_from(Self::with(names))
                .into_iter()
                .map(|(name, _)| name.to_string_lossy().into_owned())
                .collect()
        }
    }

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
        let mut host = reserved.to_vec();
        host.push("YON_PUBLIC_SETTING");
        let names = HostEnvironment::passed(&policy, &host);
        assert_eq!(names, ["YON_PUBLIC_SETTING"]);
    }

    #[test]
    fn only_the_baseline_passes_by_default() {
        let names = HostEnvironment::passed(
            &EnvironmentPolicy::default(),
            &["PATH", "HOME", "HD_REGION", "AWS_SECRET_ACCESS_KEY"],
        );
        assert_eq!(names, ["PATH"]);
    }

    #[test]
    fn configured_names_and_prefixes_pass_and_nothing_else() {
        let allowlist =
            EnvironmentAllowlist::parse(&["HOME", "HD_*"]).expect("valid configuration");
        let mut policy = EnvironmentPolicy::default();
        policy.include(&allowlist);
        let names = HostEnvironment::passed(
            &policy,
            &[
                "PATH",
                "HOME",
                "HD_REGION",
                "HD_",
                "HDX",
                "OTHER_SECRET",
                "HOMEPAGE",
                "YON_RESOURCE_TOKEN",
            ],
        );
        assert_eq!(names, ["PATH", "HOME", "HD_REGION", "HD_"]);
    }

    #[test]
    fn passed_variables_keep_their_values_unchanged() {
        let allowlist = EnvironmentAllowlist::parse(&["HD_*"]).expect("valid configuration");
        let mut policy = EnvironmentPolicy::default();
        policy.include(&allowlist);
        let passed = policy.variables_from(HostEnvironment::with(&["HD_TOKEN"]));
        assert_eq!(
            passed,
            [(
                OsString::from("HD_TOKEN"),
                OsString::from(HostEnvironment::VALUE)
            )]
        );
    }
}
