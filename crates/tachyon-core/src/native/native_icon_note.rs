use super::config::NativeApplication;
use std::path::Path;
use tachyon_contracts::NativeTarget;

pub(super) struct NativeIconNote;

impl NativeIconNote {
    const WINDOWS_EDGE: u32 = 256;
    const UNNAMED: [&'static str; 2] = [
        "client/manifest.json names no PNG or JPEG icon. SVG icons reach only the web",
        "client/manifest.json names no PNG or JPEG icon of at most 256 pixels, which the .exe file needs",
    ];

    pub(super) fn note(
        application: &NativeApplication,
        target: NativeTarget,
        web_bundle: &Path,
    ) -> Option<String> {
        let platform = super::compiler::native_target_directory(target);
        Self::chosen(application, target).map_or_else(
            || Some(Self::unnamed(platform, target)),
            |source| Self::missing(platform, source, web_bundle),
        )
    }

    pub(super) fn chosen(application: &NativeApplication, target: NativeTarget) -> Option<&str> {
        [
            application.largest_icon(),
            application.icon_within(Self::WINDOWS_EDGE),
        ][usize::from(target == NativeTarget::Windows)]
    }

    fn unnamed(platform: &str, target: NativeTarget) -> String {
        let reason = Self::UNNAMED[usize::from(target == NativeTarget::Windows)];
        format!(
            "Note: the {platform} app has no icon, so the system shows its generic icon. {reason}. Add a PNG icon to the manifest's icons, such as 512x512 and 192x192."
        )
    }

    fn missing(platform: &str, source: &str, web_bundle: &Path) -> Option<String> {
        let path = web_bundle.join(source.trim_start_matches('/'));
        (!path.is_file()).then(|| {
            format!(
                "Note: the {platform} app has no icon. client/manifest.json names the icon {source}, but the built client bundle has no such file."
            )
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::NativeIconNote;
    use crate::native::config::NativeApplication;
    use std::fs;
    use tachyon_contracts::NativeTarget;

    struct IconNoteTests;

    impl IconNoteTests {
        const TARGETS: [NativeTarget; 5] = [
            NativeTarget::Macos,
            NativeTarget::Ios,
            NativeTarget::Android,
            NativeTarget::Windows,
            NativeTarget::Linux,
        ];

        fn application(icons: &serde_json::Value) -> NativeApplication {
            let root = tempfile::tempdir().expect("project");
            fs::create_dir_all(root.path().join("client")).expect("client");
            let manifest = serde_json::json!({ "icons": icons });
            fs::write(
                root.path().join("client/manifest.json"),
                manifest.to_string(),
            )
            .expect("manifest");
            NativeApplication::discover(root.path()).expect("application")
        }
    }

    #[test]
    fn an_app_with_only_an_svg_icon_gets_a_note_on_every_native_target() {
        let svg = IconNoteTests::application(&serde_json::json!([
            {"src": "/shared/assets/icon.svg", "sizes": "any", "type": "image/svg+xml"}
        ]));
        let bundle = tempfile::tempdir().expect("bundle");
        for target in IconNoteTests::TARGETS {
            let note = NativeIconNote::note(&svg, target, bundle.path()).expect("note");
            let platform = crate::native::native_target_directory(target);
            assert!(
                note.contains(&format!("the {platform} app has no icon")),
                "{note}"
            );
            assert!(note.contains("names no PNG or JPEG icon"), "{note}");
        }
    }

    #[test]
    fn a_named_icon_missing_from_the_bundle_gets_a_note_and_a_present_one_does_not() {
        let named = IconNoteTests::application(&serde_json::json!([
            {"src": "/icons/app-512.png", "sizes": "512x512"},
            {"src": "/icons/app-192.png", "sizes": "192x192"}
        ]));
        let bundle = tempfile::tempdir().expect("bundle");
        for target in IconNoteTests::TARGETS {
            let note = NativeIconNote::note(&named, target, bundle.path()).expect("note");
            assert!(
                note.contains("but the built client bundle has no such file"),
                "{note}"
            );
        }
        fs::create_dir_all(bundle.path().join("icons")).expect("icons");
        fs::write(bundle.path().join("icons/app-512.png"), b"png").expect("512");
        fs::write(bundle.path().join("icons/app-192.png"), b"png").expect("192");
        for target in IconNoteTests::TARGETS {
            assert_eq!(NativeIconNote::note(&named, target, bundle.path()), None);
        }
    }

    #[test]
    fn each_target_takes_the_icon_its_build_uses() {
        let named = IconNoteTests::application(&serde_json::json!([
            {"src": "/icons/app-192.png", "sizes": "192x192"},
            {"src": "/icons/app.svg", "sizes": "any"},
            {"src": "/icons/app-1024.png", "sizes": "1024x1024"},
            {"src": "/icons/app-256.png", "sizes": "256x256"}
        ]));
        assert_eq!(
            named.icons,
            vec![
                (1024, String::from("/icons/app-1024.png")),
                (256, String::from("/icons/app-256.png")),
                (192, String::from("/icons/app-192.png")),
            ]
        );
        for target in IconNoteTests::TARGETS {
            let expected = ["/icons/app-1024.png", "/icons/app-256.png"]
                [usize::from(target == NativeTarget::Windows)];
            assert_eq!(
                NativeIconNote::chosen(&named, target),
                Some(expected),
                "{target:?}"
            );
        }
        let large = IconNoteTests::application(
            &serde_json::json!([{"src": "/icons/app-512.png", "sizes": "512x512"}]),
        );
        assert_eq!(NativeIconNote::chosen(&large, NativeTarget::Windows), None);
        let note = NativeIconNote::note(
            &large,
            NativeTarget::Windows,
            std::path::Path::new("/missing"),
        )
        .expect("note");
        assert!(note.contains("of at most 256 pixels"), "{note}");
    }

    #[test]
    fn a_new_app_names_png_icons_that_every_native_target_uses() {
        let root = tempfile::tempdir().expect("workspace");
        let app = root.path().join("icon-app");
        crate::Scaffold::create(&app, None).expect("scaffold");
        let application = NativeApplication::discover(&app).expect("application");
        assert_eq!(
            application.icons,
            vec![
                (512, String::from("/shared/assets/icon-512.png")),
                (192, String::from("/shared/assets/icon-192.png")),
            ]
        );
        for (edge, source) in &application.icons {
            let png = fs::read(app.join("client").join(source.trim_start_matches('/')))
                .expect("icon file");
            assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
            let width = u32::from_be_bytes(png[16..20].try_into().expect("width"));
            let height = u32::from_be_bytes(png[20..24].try_into().expect("height"));
            assert_eq!([width, height], [*edge; 2], "{source}");
        }
        let client = app.join("client");
        for target in IconNoteTests::TARGETS {
            assert_eq!(NativeIconNote::note(&application, target, &client), None);
        }
    }
}
