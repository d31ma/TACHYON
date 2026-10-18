use super::system_version::SystemVersion;
use serde::Deserialize;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct MinimumSystem {
    ios: Option<String>,
    macos: Option<String>,
    android: Option<u32>,
    windows: Option<String>,
    linux: Option<String>,
    #[serde(skip)]
    raised: Option<(NativeTarget, SystemVersion)>,
}

impl MinimumSystem {
    const MOST_ANDROID_LEVEL: u32 = 99;
    const PROBLEM: &str = "Manifest tac.application.minimumSystem takes ios, macos, windows and linux as versions such as \"18.2\", and android as an API level such as 26.";

    pub(super) fn problem(&self) -> Option<&'static str> {
        let versions_parse = [&self.ios, &self.macos, &self.windows, &self.linux]
            .into_iter()
            .flatten()
            .all(|text| SystemVersion::parse(text).is_some());
        let level_fits = self
            .android
            .is_none_or(|level| (1..=Self::MOST_ANDROID_LEVEL).contains(&level));
        let valid = [versions_parse, level_fits] == [true, true];
        (!valid).then_some(Self::PROBLEM)
    }

    pub(super) fn declared(&self, target: NativeTarget) -> Option<SystemVersion> {
        self.entries()
            .into_iter()
            .find(|(entry, _)| *entry == target)
            .and_then(|(_, version)| version)
    }

    pub(super) fn floor(&self, target: NativeTarget) -> SystemVersion {
        let raised = self
            .raised
            .filter(|(raised, _)| *raised == target)
            .map(|(_, version)| version);
        self.declared(target)
            .or(raised)
            .unwrap_or_else(|| super::target_floor::TargetFloor::of(target).base)
    }

    pub(super) fn with_floor(mut self, target: NativeTarget, floor: SystemVersion) -> Self {
        self.raised = Some((target, floor));
        self
    }

    fn entries(&self) -> [(NativeTarget, Option<SystemVersion>); 5] {
        let level = self.android.map(|level| SystemVersion::new(level, 0));
        [
            (NativeTarget::Ios, Self::version(self.ios.as_deref())),
            (NativeTarget::Macos, Self::version(self.macos.as_deref())),
            (NativeTarget::Android, level),
            (
                NativeTarget::Windows,
                Self::version(self.windows.as_deref()),
            ),
            (NativeTarget::Linux, Self::version(self.linux.as_deref())),
        ]
    }

    fn version(text: Option<&str>) -> Option<SystemVersion> {
        text.and_then(SystemVersion::parse)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::MinimumSystem;
    use crate::native::system_version::SystemVersion;
    use tachyon_contracts::NativeTarget;

    fn parsed(source: &str) -> MinimumSystem {
        serde_json::from_str(source).expect("minimum system")
    }

    #[test]
    fn each_target_reads_its_own_member() {
        let minimum =
            parsed(r#"{"ios":"18.2","macos":"15","android":28,"windows":"119.0","linux":"2.46"}"#);
        assert_eq!(minimum.problem(), None);
        assert_eq!(
            minimum.declared(NativeTarget::Ios),
            Some(SystemVersion::new(18, 2))
        );
        assert_eq!(
            minimum.declared(NativeTarget::Macos),
            Some(SystemVersion::new(15, 0))
        );
        assert_eq!(
            minimum.declared(NativeTarget::Android),
            Some(SystemVersion::new(28, 0))
        );
        assert_eq!(
            minimum.declared(NativeTarget::Windows),
            Some(SystemVersion::new(119, 0))
        );
        assert_eq!(
            minimum.declared(NativeTarget::Linux),
            Some(SystemVersion::new(2, 46))
        );
    }

    #[test]
    fn an_undeclared_target_keeps_the_host_floor() {
        let minimum = MinimumSystem::default();
        assert_eq!(minimum.declared(NativeTarget::Ios), None);
        assert_eq!(
            minimum.floor(NativeTarget::Ios),
            crate::native::target_floor::TargetFloor::of(NativeTarget::Ios).base
        );
        let raised = minimum.with_floor(NativeTarget::Android, SystemVersion::new(30, 0));
        assert_eq!(
            raised.floor(NativeTarget::Android),
            SystemVersion::new(30, 0)
        );
    }

    #[test]
    fn malformed_values_and_unknown_members_are_refused() {
        for source in [
            r#"{"ios":"18.x"}"#,
            r#"{"macos":"15.2.1"}"#,
            r#"{"windows":""}"#,
            r#"{"android":0}"#,
            r#"{"android":100}"#,
        ] {
            assert!(parsed(source).problem().is_some(), "{source}");
        }
        assert!(serde_json::from_str::<MinimumSystem>(r#"{"ipados":"18.2"}"#).is_err());
        assert!(serde_json::from_str::<MinimumSystem>(r#"{"android":"26"}"#).is_err());
    }
}
