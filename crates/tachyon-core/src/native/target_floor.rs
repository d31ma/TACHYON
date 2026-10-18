use super::system_version::SystemVersion;
use crate::Failure;
use std::collections::BTreeSet;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Copy, Debug)]
pub(super) struct TargetFloor {
    target: NativeTarget,
    pub(super) base: SystemVersion,
    garbage_collection: Option<SystemVersion>,
    platform: &'static str,
    system: &'static str,
    shown: &'static str,
    newest_major: u32,
    startup_check: &'static str,
    web_view_note: Option<&'static str>,
}

impl TargetFloor {
    const VERSION: &'static str = "\"{version}\"";
    const IOS: Self = Self {
        target: NativeTarget::Ios,
        base: SystemVersion::new(17, 0),
        garbage_collection: Some(SystemVersion::new(18, 2)),
        platform: "iOS",
        system: "iOS {version}",
        shown: Self::VERSION,
        newest_major: u32::MAX,
        startup_check: "",
        web_view_note: None,
    };
    const MACOS: Self = Self {
        target: NativeTarget::Macos,
        base: SystemVersion::new(13, 0),
        garbage_collection: Some(SystemVersion::new(15, 2)),
        platform: "macOS",
        system: "macOS {version}",
        ..Self::IOS
    };
    const ANDROID: Self = Self {
        target: NativeTarget::Android,
        base: SystemVersion::new(26, 0),
        garbage_collection: None,
        platform: "Android",
        system: "Android API level {major}",
        shown: "{major}",
        newest_major: super::android::AndroidHostGenerator::COMPILE_SDK,
        startup_check: "",
        web_view_note: Some(
            "Note: {names} services need WebAssembly garbage collection, which Android System WebView 119 or newer runs. Google Play updates the web view apart from Android, so the Android app keeps its API level; an older web view shows the service's own error (ADR 0033).",
        ),
    };
    const WINDOWS: Self = Self {
        target: NativeTarget::Windows,
        base: SystemVersion::new(86, 0),
        garbage_collection: Some(SystemVersion::new(119, 0)),
        platform: "Windows",
        system: "the WebView2 Runtime {version}",
        startup_check: ", and it checks that at startup",
        ..Self::IOS
    };
    const LINUX: Self = Self {
        target: NativeTarget::Linux,
        base: SystemVersion::new(2, 40),
        garbage_collection: Some(SystemVersion::new(2, 46)),
        platform: "Linux",
        system: "WebKitGTK {version}",
        ..Self::WINDOWS
    };
    const ALL: [Self; 5] = [
        Self::IOS,
        Self::MACOS,
        Self::ANDROID,
        Self::WINDOWS,
        Self::LINUX,
    ];

    pub(super) fn of(target: NativeTarget) -> Self {
        Self::ALL
            .into_iter()
            .find(|floor| floor.target == target)
            .unwrap_or(Self::IOS)
    }

    pub(super) fn needed(self, languages: &BTreeSet<&'static str>) -> SystemVersion {
        self.garbage_collection
            .filter(|_| !languages.is_empty())
            .map_or(self.base, |floor| floor.max(self.base))
    }

    pub(super) fn note(
        self,
        declared: bool,
        needed: SystemVersion,
        languages: &BTreeSet<&'static str>,
    ) -> Option<String> {
        let raised = [declared, needed > self.base] == [false, true];
        raised
            .then(|| self.raised_note(needed, languages))
            .or_else(|| self.web_view(languages))
    }

    pub(super) fn check(
        self,
        declared: SystemVersion,
        needed: SystemVersion,
        languages: &BTreeSet<&'static str>,
    ) -> Result<(), Failure> {
        let problems = [
            (declared < self.base, self.below_host(declared)),
            (
                declared < needed,
                self.below_languages(declared, needed, languages),
            ),
            (
                declared.major() > self.newest_major,
                self.above_newest(declared),
            ),
        ];
        problems
            .into_iter()
            .find(|(failed, _)| *failed)
            .map_or(Ok(()), |(_, message)| {
                Err(super::config::config_failure(&message))
            })
    }

    fn raised_note(self, needed: SystemVersion, languages: &BTreeSet<&'static str>) -> String {
        let names = Self::listed(languages);
        let platform = self.platform;
        let wanted = Self::filled(self.system, needed);
        let checked = self.startup_check;
        let member = self.member();
        format!(
            "Note: {names} services need WebAssembly garbage collection, so the {platform} app needs {wanted} or newer{checked} (ADR 0033). Set {member} to choose the floor yourself."
        )
    }

    fn web_view(self, languages: &BTreeSet<&'static str>) -> Option<String> {
        let names = Self::listed(languages);
        self.web_view_note
            .filter(|_| !languages.is_empty())
            .map(|note| note.replace("{names}", &names))
    }

    fn below_host(self, declared: SystemVersion) -> String {
        let member = self.member();
        let shown = Self::filled(self.shown, declared);
        let oldest = Self::filled(self.system, self.base);
        let platform = self.platform;
        format!(
            "Manifest {member} is {shown}, below {oldest}, the oldest system the Tachyon {platform} app supports."
        )
    }

    fn below_languages(
        self,
        declared: SystemVersion,
        needed: SystemVersion,
        languages: &BTreeSet<&'static str>,
    ) -> String {
        let member = self.member();
        let shown = Self::filled(self.shown, declared);
        let names = Self::listed(languages);
        let wanted = Self::filled(self.system, needed);
        let raise = Self::filled(self.shown, needed);
        format!(
            "Manifest {member} is {shown}, but {names} services need {wanted} or newer for WebAssembly garbage collection (ADR 0033). Raise it to {raise}, or remove it so the build chooses."
        )
    }

    fn above_newest(self, declared: SystemVersion) -> String {
        let member = self.member();
        let shown = Self::filled(self.shown, declared);
        let newest = self.newest_major;
        let platform = self.platform;
        format!(
            "Manifest {member} is {shown}, above API level {newest}, the newest the Tachyon {platform} app compiles against."
        )
    }

    fn member(self) -> String {
        let key = super::compiler::native_target_directory(self.target);
        format!("tac.application.minimumSystem.{key}")
    }

    fn filled(template: &str, version: SystemVersion) -> String {
        template
            .replace("{version}", &version.to_string())
            .replace("{major}", &version.major().to_string())
    }

    fn listed(languages: &BTreeSet<&'static str>) -> String {
        let names: Vec<&str> = languages.iter().copied().collect();
        let (last, rest) = names.split_last().unwrap_or((&"", &[]));
        [rest.join(", "), (*last).to_owned()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<String>>()
            .join(" and ")
    }
}

#[cfg(test)]
mod tests {
    use super::TargetFloor;
    use std::collections::BTreeSet;
    use tachyon_contracts::NativeTarget;

    #[test]
    fn every_target_has_its_own_floor() {
        for target in [
            NativeTarget::Ios,
            NativeTarget::Macos,
            NativeTarget::Android,
            NativeTarget::Windows,
            NativeTarget::Linux,
        ] {
            assert_eq!(TargetFloor::of(target).target, target);
        }
    }

    #[test]
    fn languages_read_as_an_english_list() {
        let listed = |names: &[&'static str]| {
            TargetFloor::listed(&names.iter().copied().collect::<BTreeSet<_>>())
        };
        assert_eq!(listed(&[]), "");
        assert_eq!(listed(&["Dart"]), "Dart");
        assert_eq!(listed(&["Java", "Dart"]), "Dart and Java");
        assert_eq!(listed(&["Kotlin", "Java", "Dart"]), "Dart, Java and Kotlin");
    }
}
