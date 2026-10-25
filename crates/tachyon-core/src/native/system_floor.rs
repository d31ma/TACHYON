use super::config::NativeApplication;
use super::minimum_system::MinimumSystem;
use super::target_floor::TargetFloor;
use crate::Failure;
use crate::tac_web::WebLanguage;
use crate::template::{ComponentDefinition, ComponentRegistry, ServiceDefinition, ServiceRegistry};
use std::collections::BTreeSet;
use tachyon_contracts::NativeTarget;

#[derive(Debug, Eq, PartialEq)]
pub(super) struct SystemFloor {
    pub(super) minimum: MinimumSystem,
    pub(super) note: Option<String>,
    pub(super) style_note: Option<String>,
}

impl SystemFloor {
    const GARBAGE_COLLECTED: [(WebLanguage, &'static str); 2] =
        [(WebLanguage::Kotlin, "Kotlin"), (WebLanguage::Java, "Java")];

    pub(super) fn for_project(
        project: &crate::Project,
        application: &NativeApplication,
        target: NativeTarget,
    ) -> Result<Self, Failure> {
        let registry = ServiceRegistry::discover(project.snapshot_root())?.for_target(Some(target));
        let styled =
            Self::has_component_styles(&ComponentRegistry::discover(project.snapshot_root())?);
        let floor = Self::resolve(&application.minimum_system, target, &registry)?;
        Ok(Self {
            style_note: TargetFloor::of(target).style_note(styled),
            ..floor
        })
    }

    fn has_component_styles(components: &ComponentRegistry) -> bool {
        components.names().iter().any(|name| {
            components
                .get(name)
                .and_then(ComponentDefinition::style_path)
                .is_some()
        })
    }

    fn resolve(
        minimum: &MinimumSystem,
        target: NativeTarget,
        registry: &ServiceRegistry,
    ) -> Result<Self, Failure> {
        let languages = registry
            .iter()
            .filter_map(|(_, service)| Self::garbage_collected_language(service))
            .collect();
        Self::for_languages(minimum, target, &languages)
    }

    fn for_languages(
        minimum: &MinimumSystem,
        target: NativeTarget,
        languages: &BTreeSet<&'static str>,
    ) -> Result<Self, Failure> {
        let floor = TargetFloor::of(target);
        let needed = floor.needed(languages);
        let declared = minimum.declared(target);
        declared.map_or(Ok(()), |declared| floor.check(declared, needed, languages))?;
        Ok(Self {
            minimum: minimum.clone().with_floor(target, needed),
            note: floor.note(declared.is_some(), needed, languages),
            style_note: None,
        })
    }

    fn garbage_collected_language(service: &ServiceDefinition) -> Option<&'static str> {
        let dart = service
            .script_path()
            .and_then(std::path::Path::extension)
            .is_some_and(|extension| extension == "dart")
            .then_some("Dart");
        let compiled = service.web_assembly().and_then(|(language, _)| {
            Self::GARBAGE_COLLECTED
                .into_iter()
                .find(|(wanted, _)| *wanted == language)
        });
        dart.or(compiled.map(|(_, name)| name))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::SystemFloor;
    use crate::native::minimum_system::MinimumSystem;
    use crate::native::system_version::SystemVersion;
    use std::collections::BTreeSet;
    use tachyon_contracts::NativeTarget;

    const ALL_TARGETS: [NativeTarget; 5] = [
        NativeTarget::Ios,
        NativeTarget::Macos,
        NativeTarget::Android,
        NativeTarget::Windows,
        NativeTarget::Linux,
    ];

    fn declared(source: &str) -> MinimumSystem {
        serde_json::from_str(source).expect("minimum system")
    }

    fn message(failure: &crate::Failure) -> String {
        failure
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect()
    }

    #[test]
    fn without_garbage_collected_services_every_target_keeps_its_host_floor() {
        for target in ALL_TARGETS {
            let floor =
                SystemFloor::for_languages(&MinimumSystem::default(), target, &BTreeSet::new())
                    .expect("floor");
            assert_eq!(
                floor.minimum.floor(target),
                crate::native::target_floor::TargetFloor::of(target).base
            );
            assert_eq!(floor.note, None, "{target:?}");
        }
    }

    #[test]
    fn garbage_collected_services_raise_the_floor_with_one_note() {
        let languages = BTreeSet::from(["Kotlin"]);
        for (target, wanted) in [
            (NativeTarget::Ios, SystemVersion::new(18, 2)),
            (NativeTarget::Macos, SystemVersion::new(15, 2)),
            (NativeTarget::Windows, SystemVersion::new(119, 0)),
            (NativeTarget::Linux, SystemVersion::new(2, 46)),
        ] {
            let floor = SystemFloor::for_languages(&MinimumSystem::default(), target, &languages)
                .expect("floor");
            assert_eq!(floor.minimum.floor(target), wanted, "{target:?}");
            let note = floor.note.expect("note");
            assert!(
                note.starts_with("Note: Kotlin services need WebAssembly garbage collection, so")
            );
            assert!(note.contains(&wanted.to_string()), "{note}");
            assert!(note.contains("(ADR 0033)"), "{note}");
        }
    }

    #[test]
    fn dart_services_raise_the_floor_to_engines_with_the_javascript_exception_tag() {
        for (languages, start) in [
            (
                BTreeSet::from(["Dart"]),
                "Note: Dart services need WebAssembly garbage collection and the JavaScript exception tag (WebAssembly.JSTag), so",
            ),
            (
                BTreeSet::from(["Dart", "Kotlin"]),
                "Note: Dart and Kotlin services need WebAssembly garbage collection and, for Dart, the JavaScript exception tag (WebAssembly.JSTag), so",
            ),
        ] {
            for (target, wanted) in [
                (NativeTarget::Ios, SystemVersion::new(18, 4)),
                (NativeTarget::Macos, SystemVersion::new(15, 4)),
                (NativeTarget::Windows, SystemVersion::new(119, 0)),
                (NativeTarget::Linux, SystemVersion::new(2, 48)),
            ] {
                let floor =
                    SystemFloor::for_languages(&MinimumSystem::default(), target, &languages)
                        .expect("floor");
                assert_eq!(floor.minimum.floor(target), wanted, "{target:?}");
                let note = floor.note.expect("note");
                assert!(note.starts_with(start), "{note}");
                assert!(note.contains(&wanted.to_string()), "{note}");
                assert!(note.contains("(ADR 0033)"), "{note}");
            }
        }
    }

    #[test]
    fn java_services_raise_the_floor_to_engines_with_exception_references() {
        for (languages, start) in [
            (
                BTreeSet::from(["Java"]),
                "Note: Java services need WebAssembly garbage collection and exception references (exnref), so",
            ),
            (
                BTreeSet::from(["Dart", "Java", "Kotlin"]),
                "Note: Dart, Java and Kotlin services need WebAssembly garbage collection, for Dart, the JavaScript exception tag (WebAssembly.JSTag) and, for Java, exception references (exnref), so",
            ),
        ] {
            for (target, wanted) in [
                (NativeTarget::Ios, SystemVersion::new(18, 4)),
                (NativeTarget::Macos, SystemVersion::new(15, 4)),
                (NativeTarget::Windows, SystemVersion::new(137, 0)),
                (NativeTarget::Linux, SystemVersion::new(2, 48)),
            ] {
                let floor =
                    SystemFloor::for_languages(&MinimumSystem::default(), target, &languages)
                        .expect("floor");
                assert_eq!(floor.minimum.floor(target), wanted, "{target:?}");
                let note = floor.note.expect("note");
                assert!(note.starts_with(start), "{note}");
                assert!(note.contains(&wanted.to_string()), "{note}");
                assert!(note.contains("(ADR 0033, ADR 0083)"), "{note}");
            }
        }
    }

    #[test]
    fn a_declared_floor_below_the_java_need_names_exception_references() {
        let minimum = declared(r#"{"ios":"18.2","windows":"119.0","linux":"2.46"}"#);
        let languages = BTreeSet::from(["Java"]);
        for (target, wanted) in [
            (
                NativeTarget::Ios,
                "is \"18.2\", but Java services need iOS 18.4 or newer for WebAssembly garbage collection and exception references (exnref) (ADR 0033, ADR 0083). Raise it to \"18.4\"",
            ),
            (
                NativeTarget::Windows,
                "is \"119.0\", but Java services need the WebView2 Runtime 137.0 or newer",
            ),
            (
                NativeTarget::Linux,
                "is \"2.46\", but Java services need WebKitGTK 2.48 or newer",
            ),
        ] {
            let failure =
                SystemFloor::for_languages(&minimum, target, &languages).expect_err("refused");
            let text = message(&failure);
            assert!(text.contains(wanted), "{text}");
            assert_eq!(failure.diagnostics()[0].code.number(), 1601);
        }
    }

    #[test]
    fn a_declared_floor_below_the_dart_need_names_the_javascript_exception_tag() {
        let minimum = declared(r#"{"ios":"18.2","linux":"2.46"}"#);
        let languages = BTreeSet::from(["Dart"]);
        for (target, wanted) in [
            (
                NativeTarget::Ios,
                "is \"18.2\", but Dart services need iOS 18.4 or newer for WebAssembly garbage collection and the JavaScript exception tag (WebAssembly.JSTag) (ADR 0033). Raise it to \"18.4\"",
            ),
            (
                NativeTarget::Linux,
                "is \"2.46\", but Dart services need WebKitGTK 2.48 or newer",
            ),
        ] {
            let failure =
                SystemFloor::for_languages(&minimum, target, &languages).expect_err("refused");
            let text = message(&failure);
            assert!(text.contains(wanted), "{text}");
            assert_eq!(failure.diagnostics()[0].code.number(), 1601);
        }
    }

    fn discovered(files: &[(&str, &str)]) -> crate::template::ServiceRegistry {
        let root = tempfile::tempdir().expect("project");
        for (relative, contents) in files {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, contents).expect("write");
        }
        crate::template::ServiceRegistry::discover(root.path()).expect("registry")
    }

    #[test]
    fn a_component_stylesheet_is_what_makes_a_project_styled() {
        let root = tempfile::tempdir().expect("project");
        let component = root.path().join("client/components/product/card");
        std::fs::create_dir_all(&component).expect("component");
        std::fs::write(component.join("tac.html"), "<p>card</p>").expect("template");
        let plain = crate::template::ComponentRegistry::discover(root.path()).expect("plain");
        assert!(!SystemFloor::has_component_styles(&plain));
        std::fs::write(component.join("tac.css"), "p { color: red; }").expect("style");
        let styled = crate::template::ComponentRegistry::discover(root.path()).expect("styled");
        assert!(SystemFloor::has_component_styles(&styled));
    }

    #[test]
    fn the_services_that_run_as_garbage_collected_webassembly_set_the_floor() {
        let registry = discovered(&[
            (
                "client/services/PagerService.dart",
                "@Service\nclass PagerService {}",
            ),
            (
                "client/services/PanelService.kt",
                "@Service\nclass PanelService {\n}\n",
            ),
            (
                "client/services/ClockService.rs",
                "#[derive(Default)]\n#[Service]\nstruct ClockService {}\n",
            ),
        ]);
        let ios = SystemFloor::resolve(
            &MinimumSystem::default(),
            NativeTarget::Ios,
            &registry.clone().for_target(Some(NativeTarget::Ios)),
        )
        .expect("ios");
        assert_eq!(
            ios.minimum.floor(NativeTarget::Ios),
            SystemVersion::new(18, 4)
        );
        assert!(
            ios.note
                .expect("note")
                .starts_with("Note: Dart and Kotlin services need")
        );
        let android = SystemFloor::resolve(
            &MinimumSystem::default(),
            NativeTarget::Android,
            &registry.for_target(Some(NativeTarget::Android)),
        )
        .expect("android");
        assert!(
            android
                .note
                .expect("note")
                .starts_with("Note: Dart services need"),
            "Kotlin is linked on Android, so only Dart needs the web view"
        );
        let plain = discovered(&[(
            "client/services/ClockService.rs",
            "#[derive(Default)]\n#[Service]\nstruct ClockService {}\n",
        )]);
        let macos = SystemFloor::resolve(
            &MinimumSystem::default(),
            NativeTarget::Macos,
            &plain.for_target(Some(NativeTarget::Macos)),
        )
        .expect("macos");
        assert_eq!(macos.note, None);
    }

    #[test]
    fn android_keeps_its_api_level_and_names_the_web_view() {
        let languages = BTreeSet::from(["Java"]);
        let floor = SystemFloor::for_languages(
            &MinimumSystem::default(),
            NativeTarget::Android,
            &languages,
        )
        .expect("floor");
        assert_eq!(
            floor.minimum.floor(NativeTarget::Android),
            SystemVersion::new(26, 0)
        );
        let note = floor.note.expect("note");
        assert!(note.contains("Note: Java services need"));
        assert!(note.contains("exception references (exnref)"), "{note}");
        assert!(note.contains("Android System WebView 137"), "{note}");
        let dart = SystemFloor::for_languages(
            &MinimumSystem::default(),
            NativeTarget::Android,
            &BTreeSet::from(["Dart"]),
        )
        .expect("floor")
        .note
        .expect("note");
        assert!(dart.contains("Android System WebView 119"), "{dart}");
        assert!(dart.contains("WebAssembly.JSTag"), "{dart}");
        assert!(!dart.contains("exnref"), "{dart}");
    }

    #[test]
    fn a_declared_floor_at_or_above_the_need_is_kept_without_a_note() {
        let minimum = declared(r#"{"ios":"26.0","linux":"2.48"}"#);
        let languages = BTreeSet::from(["Dart"]);
        for target in [NativeTarget::Ios, NativeTarget::Linux] {
            let floor = SystemFloor::for_languages(&minimum, target, &languages).expect("floor");
            assert_eq!(floor.minimum.floor(target), minimum.floor(target));
            assert_eq!(floor.note, None);
        }
    }

    #[test]
    fn a_declared_floor_below_the_need_names_the_languages() {
        let minimum = declared(r#"{"macos":"14.0"}"#);
        let languages = BTreeSet::from(["Dart", "Kotlin"]);
        let failure = SystemFloor::for_languages(&minimum, NativeTarget::Macos, &languages)
            .expect_err("refused");
        let text = message(&failure);
        assert!(text.contains("tac.application.minimumSystem.macos is \"14.0\""));
        assert!(text.contains("Dart and Kotlin services need macOS 15.4 or newer"));
        assert_eq!(failure.diagnostics()[0].code.number(), 1601);
    }

    #[test]
    fn a_declared_floor_below_the_host_or_above_android_is_refused() {
        let cases = [
            (
                r#"{"ios":"16.4"}"#,
                NativeTarget::Ios,
                "below iOS 17.4, the oldest system the Tachyon iOS app supports",
            ),
            (
                r#"{"android":24}"#,
                NativeTarget::Android,
                "is 24, below Android API level 26",
            ),
            (
                r#"{"android":99}"#,
                NativeTarget::Android,
                "is 99, above API level 35",
            ),
            (
                r#"{"linux":"2.38"}"#,
                NativeTarget::Linux,
                "below WebKitGTK 2.40",
            ),
        ];
        for (source, target, wanted) in cases {
            let failure = SystemFloor::for_languages(&declared(source), target, &BTreeSet::new())
                .expect_err("refused");
            let text = message(&failure);
            assert!(text.contains(wanted), "{text}");
        }
    }
}
