use super::system_version::SystemVersion;
use crate::Failure;
use std::collections::BTreeSet;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Copy, Debug)]
pub(super) struct TargetFloor {
    target: NativeTarget,
    pub(super) base: SystemVersion,
    garbage_collection: Option<SystemVersion>,
    exception_references: Option<SystemVersion>,
    platform: &'static str,
    system: &'static str,
    shown: &'static str,
    newest_major: u32,
    startup_check: &'static str,
    web_view_note: Option<&'static str>,
    style_note: Option<&'static str>,
    web_view_versions: [u32; 2],
}

impl TargetFloor {
    const VERSION: &'static str = "\"{version}\"";
    const EXCEPTION_LANGUAGE: &'static str = "Java";
    const IOS: Self = Self {
        target: NativeTarget::Ios,
        base: SystemVersion::new(17, 4),
        garbage_collection: Some(SystemVersion::new(18, 2)),
        exception_references: Some(SystemVersion::new(18, 4)),
        platform: "iOS",
        system: "iOS {version}",
        shown: Self::VERSION,
        newest_major: u32::MAX,
        startup_check: "",
        web_view_note: None,
        style_note: None,
        web_view_versions: [0, 0],
    };
    const MACOS: Self = Self {
        target: NativeTarget::Macos,
        base: SystemVersion::new(13, 0),
        garbage_collection: Some(SystemVersion::new(15, 2)),
        exception_references: Some(SystemVersion::new(15, 4)),
        platform: "macOS",
        system: "macOS {version}",
        style_note: Some(
            "Note: component styles use CSS @scope, which the macOS web view runs from Safari 17.4. Safari updates apart from macOS, so the macOS app keeps its floor; a Mac with an older Safari shows the components without their styles (ADR 0108).",
        ),
        ..Self::IOS
    };
    const ANDROID: Self = Self {
        target: NativeTarget::Android,
        base: SystemVersion::new(26, 0),
        garbage_collection: None,
        exception_references: None,
        platform: "Android",
        system: "Android API level {major}",
        shown: "{major}",
        newest_major: super::android::AndroidHostGenerator::COMPILE_SDK,
        startup_check: "",
        web_view_note: Some(
            "Note: {names} services need {features}, which Android System WebView {web_view} or newer runs. Google Play updates the web view apart from Android, so the Android app keeps its API level; an older web view shows the service's own error ({decisions}).",
        ),
        style_note: Some(
            "Note: component styles use CSS @scope, which Android System WebView 118 or newer runs. Google Play updates the web view apart from Android, so the Android app keeps its API level; an older web view shows the components without their styles (ADR 0108).",
        ),
        web_view_versions: [119, 137],
    };
    const WINDOWS: Self = Self {
        target: NativeTarget::Windows,
        base: SystemVersion::new(118, 0),
        garbage_collection: Some(SystemVersion::new(119, 0)),
        exception_references: Some(SystemVersion::new(137, 0)),
        platform: "Windows",
        system: "the WebView2 Runtime {version}",
        startup_check: ", and it checks that at startup",
        ..Self::IOS
    };
    const LINUX: Self = Self {
        target: NativeTarget::Linux,
        base: SystemVersion::new(2, 40),
        garbage_collection: Some(SystemVersion::new(2, 46)),
        exception_references: Some(SystemVersion::new(2, 48)),
        platform: "Linux",
        system: "WebKitGTK {version}",
        style_note: Some(
            "Note: component styles use CSS @scope, which WebKitGTK 2.44 or newer runs. WebKitGTK comes with the Linux distribution, apart from the app, so the Linux app keeps its 2.40 floor; an older WebKitGTK shows the components without their styles (ADR 0108).",
        ),
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
        let exceptions = self
            .exception_references
            .filter(|_| Self::needs_exception_references(languages));
        self.garbage_collection
            .filter(|_| !languages.is_empty())
            .into_iter()
            .chain(exceptions)
            .fold(self.base, SystemVersion::max)
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

    pub(super) fn style_note(self, styled: bool) -> Option<String> {
        self.style_note.filter(|_| styled).map(str::to_owned)
    }

    fn raised_note(self, needed: SystemVersion, languages: &BTreeSet<&'static str>) -> String {
        let names = Self::listed(languages);
        let features = Self::features(languages);
        let platform = self.platform;
        let wanted = Self::filled(self.system, needed);
        let checked = self.startup_check;
        let decisions = Self::decisions(languages);
        let member = self.member();
        format!(
            "Note: {names} services need {features}, so the {platform} app needs {wanted} or newer{checked} ({decisions}). Set {member} to choose the floor yourself."
        )
    }

    fn web_view(self, languages: &BTreeSet<&'static str>) -> Option<String> {
        let names = Self::listed(languages);
        let [garbage_collection, exception_references] = self.web_view_versions;
        let web_view = if Self::needs_exception_references(languages) {
            exception_references
        } else {
            garbage_collection
        };
        self.web_view_note
            .filter(|_| !languages.is_empty())
            .map(|note| {
                note.replace("{names}", &names)
                    .replace("{features}", Self::features(languages))
                    .replace("{web_view}", &web_view.to_string())
                    .replace("{decisions}", Self::decisions(languages))
            })
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
        let features = Self::features(languages);
        let wanted = Self::filled(self.system, needed);
        let decisions = Self::decisions(languages);
        let raise = Self::filled(self.shown, needed);
        format!(
            "Manifest {member} is {shown}, but {names} services need {wanted} or newer for {features} ({decisions}). Raise it to {raise}, or remove it so the build chooses."
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

    fn needs_exception_references(languages: &BTreeSet<&'static str>) -> bool {
        languages.contains(Self::EXCEPTION_LANGUAGE)
    }

    fn features(languages: &BTreeSet<&'static str>) -> &'static str {
        match (Self::needs_exception_references(languages), languages.len()) {
            (false, _) => "WebAssembly garbage collection",
            (true, 1) => "WebAssembly garbage collection and exception references (exnref)",
            (true, _) => {
                "WebAssembly garbage collection and, for Java, exception references (exnref)"
            }
        }
    }

    fn decisions(languages: &BTreeSet<&'static str>) -> &'static str {
        if Self::needs_exception_references(languages) {
            "ADR 0033, ADR 0083"
        } else {
            "ADR 0033"
        }
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
    fn only_hosts_whose_web_view_updates_apart_note_component_styles() {
        for target in [
            NativeTarget::Macos,
            NativeTarget::Android,
            NativeTarget::Linux,
        ] {
            let note = TargetFloor::of(target).style_note(true).unwrap_or_default();
            assert!(note.contains("CSS @scope"), "{target:?}: {note}");
            assert_eq!(TargetFloor::of(target).style_note(false), None);
        }
        for target in [NativeTarget::Ios, NativeTarget::Windows] {
            assert_eq!(TargetFloor::of(target).style_note(true), None, "{target:?}");
        }
    }

    #[test]
    fn the_linux_note_names_the_first_webkitgtk_that_scopes_styles() {
        let linux = TargetFloor::of(NativeTarget::Linux);
        let note = linux.style_note(true).unwrap_or_default();
        assert!(note.contains("WebKitGTK 2.44 or newer"), "{note}");
        assert!(note.contains("keeps its 2.40 floor"), "{note}");
        assert_eq!(linux.base, super::SystemVersion::new(2, 40));
    }

    #[test]
    fn the_floors_reach_engines_that_scope_component_styles() {
        assert_eq!(
            TargetFloor::of(NativeTarget::Ios).base,
            super::SystemVersion::new(17, 4)
        );
        assert_eq!(
            TargetFloor::of(NativeTarget::Windows).base,
            super::SystemVersion::new(118, 0)
        );
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
