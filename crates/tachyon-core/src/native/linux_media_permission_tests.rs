#![allow(clippy::expect_used)]

use super::browser_api_permissions::BrowserApiPermissions;

struct LinuxMediaPermissionTests;

impl LinuxMediaPermissionTests {
    const SCRIPT: &'static str = r"if (!navigator.mediaDevices) return 'no-media-devices';
const ask = (constraints) => navigator.mediaDevices.getUserMedia(constraints).then((stream) => {
  const kinds = stream.getTracks().map((track) => track.kind).sort().join('+');
  stream.getTracks().forEach((track) => track.stop());
  return kinds;
}, (error) => error.name);
const results = [];
results.push(await ask({video: true}));
results.push(await ask({audio: true}));
results.push(await ask({audio: true, video: true}));
results.push('screen:' + typeof navigator.mediaDevices.getDisplayMedia);
return results.join(',');";
    #[cfg(target_os = "linux")]
    const CAMERA: &'static str = "video,NotAllowedError,NotAllowedError,screen:undefined";
    #[cfg(target_os = "linux")]
    const BOTH: &'static str = "video,audio,audio+video,screen:undefined";
    #[cfg(target_os = "linux")]
    const MICROPHONE: &'static str = "NotAllowedError,audio,NotAllowedError,screen:undefined";
    #[cfg(target_os = "linux")]
    const SCREEN_WITHOUT_PORTAL: &'static str =
        "NotAllowedError,NotAllowedError,NotAllowedError,screen:undefined";
    #[cfg(target_os = "linux")]
    const SCREEN_WITH_PORTAL: &'static str =
        "NotAllowedError,NotAllowedError,NotAllowedError,screen:function";
    #[cfg(target_os = "linux")]
    const CAMERA_WITH_PORTAL: &'static str =
        "video,NotAllowedError,NotAllowedError,screen:undefined";
    #[cfg(target_os = "linux")]
    const NOTHING: &'static str = "no-media-devices";

    fn permissions(granted: &[&str]) -> BrowserApiPermissions {
        let granted: Vec<String> = granted.iter().map(|api| String::from(*api)).collect();
        BrowserApiPermissions::declared(&granted, &std::collections::BTreeMap::new())
    }

    fn host(granted: &[&str]) -> String {
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.permissions = Self::permissions(granted);
        super::linux::c_source(
            &application,
            &super::linux_host_tests::LinuxHostTests::index(),
        )
    }

    fn fixture(expected: &str, fake_portal: bool) -> String {
        include_str!("linux_media_permission_fixture.c")
            .replace(
                "__MEDIA_FAKE_PORTAL__",
                ["FALSE", "TRUE"][usize::from(fake_portal)],
            )
            .replace(
                "__MEDIA_SCRIPT__",
                &super::host::c_string_escape(Self::SCRIPT),
            )
            .replace(
                "__MEDIA_EXPECTED__",
                &super::host::c_string_escape(expected),
            )
    }

    #[cfg(target_os = "linux")]
    async fn run(granted: &[&str], expected: &str, fake_portal: bool) {
        let root = tempfile::tempdir().expect("fixture");
        let source = super::native_menu_action_tests::MenuActionFixture::stage(root.path());
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.permissions = Self::permissions(granted);
        let executable = super::linux_host_tests::LinuxHostTests::compile_application(
            root.path(),
            &source,
            &application,
            &Self::fixture(expected, fake_portal),
        )
        .await;
        let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
            .await
            .expect("actual generated Linux host media permissions");
        assert!(
            output.contains("linux media permissions passed"),
            "{output}"
        );
        println!("{output}");
    }
}

#[test]
fn the_linux_host_turns_on_media_capture_only_for_a_media_grant() {
    let source = LinuxMediaPermissionTests::host(&["microphone"]);
    assert!(
        source.contains(
            "webkit_settings_set_enable_media_stream(settings, tachyon_media_granted());"
        )
    );
    assert!(source.contains(
        "webkit_settings_set_media_playback_requires_user_gesture(settings, !tachyon_media_granted());"
    ));
    assert!(source.contains("if (strcmp(api, \"microphone\") == 0) return TRUE;"));
    assert!(source.contains("return tachyon_browser_api_granted(\"camera\") || tachyon_browser_api_granted(\"microphone\") || tachyon_browser_api_granted(\"screen\");"));
    let nothing = LinuxMediaPermissionTests::host(&[]);
    assert!(!nothing.contains("if (strcmp(api, \"microphone\") == 0) return TRUE;"));
    let camera = LinuxMediaPermissionTests::host(&["camera"]);
    assert!(camera.contains("if (strcmp(api, \"camera\") == 0) return TRUE;"));
    assert!(!camera.contains("if (strcmp(api, \"microphone\") == 0) return TRUE;"));
}

#[test]
fn the_linux_host_allows_each_capture_kind_only_with_its_grant() {
    let source = LinuxMediaPermissionTests::host(&["microphone"]);
    for (asked, api) in [
        (
            "webkit_user_media_permission_is_for_audio_device",
            "microphone",
        ),
        ("webkit_user_media_permission_is_for_video_device", "camera"),
        (
            "webkit_user_media_permission_is_for_display_device",
            "screen",
        ),
    ] {
        assert!(source.contains(&format!(
            "tachyon_media_kind_allowed({asked}(request), \"{api}\")"
        )));
    }
    assert!(source.contains("if (WEBKIT_IS_USER_MEDIA_PERMISSION_REQUEST(request)) return tachyon_user_media_granted(WEBKIT_USER_MEDIA_PERMISSION_REQUEST(request));"));
    assert!(source.contains(
        "if (WEBKIT_IS_DEVICE_INFO_PERMISSION_REQUEST(request)) return tachyon_media_granted();"
    ));
    assert!(source.contains("if (tachyon_trusted_view(view) && tachyon_permission_granted(request)) webkit_permission_request_allow(request);"));
}

#[test]
fn the_linux_host_hides_screen_sharing_without_the_grant_or_a_screen_cast_portal() {
    let source = LinuxMediaPermissionTests::host(&["screen"]);
    assert!(
        source.contains(
            "TACHYON_NATIVE_SHIM, TACHYON_FILE_PICKER_PAGE, tachyon_screen_share_page());"
        )
    );
    assert!(
        source.contains(
            "g_variant_new(\"(ss)\", \"org.freedesktop.portal.ScreenCast\", \"version\")"
        )
    );
    assert!(source.contains("return tachyon_browser_api_granted(\"screen\") && tachyon_screen_cast_ready() ? \"\" : TACHYON_SCREEN_SHARE_HIDDEN;"));
    assert!(source.contains("delete MediaDevices.prototype.getDisplayMedia;"));
}

#[test]
fn the_media_permission_fixture_fills_every_placeholder() {
    let fixture = LinuxMediaPermissionTests::fixture("video", true);
    assert!(!fixture.contains("__MEDIA_"));
    assert!(fixture.contains("if (TRUE) tachyon_fake_portal_start();"));
    assert!(
        LinuxMediaPermissionTests::fixture("video", false)
            .contains("if (FALSE) tachyon_fake_portal_start();")
    );
    assert!(fixture.contains("webkit_settings_set_enable_mock_capture_devices"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, a session bus and a display; run explicitly on the Linux lane"]
async fn linux_media_capture_follows_the_manifest_grants_in_the_real_web_view() {
    LinuxMediaPermissionTests::run(&["camera"], LinuxMediaPermissionTests::CAMERA, false).await;
    LinuxMediaPermissionTests::run(
        &["microphone"],
        LinuxMediaPermissionTests::MICROPHONE,
        false,
    )
    .await;
    LinuxMediaPermissionTests::run(
        &["camera", "microphone"],
        LinuxMediaPermissionTests::BOTH,
        false,
    )
    .await;
    LinuxMediaPermissionTests::run(&[], LinuxMediaPermissionTests::NOTHING, false).await;
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, a session bus and a display; run explicitly on the Linux lane"]
async fn linux_screen_sharing_needs_the_screen_grant_and_a_screen_cast_portal() {
    LinuxMediaPermissionTests::run(
        &["screen"],
        LinuxMediaPermissionTests::SCREEN_WITH_PORTAL,
        true,
    )
    .await;
    LinuxMediaPermissionTests::run(
        &["camera"],
        LinuxMediaPermissionTests::CAMERA_WITH_PORTAL,
        true,
    )
    .await;
    LinuxMediaPermissionTests::run(
        &["screen"],
        LinuxMediaPermissionTests::SCREEN_WITHOUT_PORTAL,
        false,
    )
    .await;
}
