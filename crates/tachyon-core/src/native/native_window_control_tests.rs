#![allow(clippy::expect_used)]

use super::config::WindowConfiguration;
use super::native_window_controls::NativeWindowControls;
use std::process::Command;

struct NativeWindowCheck;

impl NativeWindowCheck {
    fn window() -> WindowConfiguration {
        WindowConfiguration {
            controls: ["minimize", "resize", "title", "fullscreen"]
                .map(String::from)
                .to_vec(),
            ..WindowConfiguration::default()
        }
    }

    fn success(command: &mut Command) {
        let output = command.output().expect("compiler");
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn windows() {
        if !Command::new("cc")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            eprintln!("skipping: C compiler is unavailable");
            return;
        }
        let root = tempfile::tempdir().expect("fixture");
        let json = include_str!("local_bundle.h")
            .split("static size_t tachyon_skip_space")
            .nth(1)
            .expect("JSON field parser");
        let json = format!("static size_t tachyon_skip_space{json}");
        let body = include_str!("../../tests/fixtures/native/windows_capability_host.c")
            .replace("__LOCAL_JSON__", &json)
            .replace(
                "__WINDOW_HOST__",
                &NativeWindowControls::windows(&Self::window()),
            );
        let source = root.path().join("host.c");
        std::fs::write(&source, body).expect("source");
        let program = root.path().join("host");
        Self::success(Command::new("cc").arg(&source).arg("-o").arg(&program));
        Self::success(&mut Command::new(&program));
    }

    fn android() {
        let root = tempfile::tempdir().expect("fixture");
        let source = root.path().join("WindowHost.java");
        let body = NativeWindowControls::android(&Self::window());
        assert!(body.contains("case \"minimize\": return true;"));
        assert!(!body.contains("case \"close\": return true;"));
        assert!(body.contains("ready.await(2,"));
        let program = format!(
            "class WindowHost extends android.app.Activity {{ private long documentGeneration; private static boolean withinByteLimit(String text, int limit) {{ return text.getBytes(java.nio.charset.StandardCharsets.UTF_8).length <= limit; }}\n{body}\n}}"
        );
        std::fs::write(&source, program).expect("source");
        let platform = std::env::var_os("ANDROID_HOME")
            .map(std::path::PathBuf::from)
            .map(|home| home.join("platforms/android-35/android.jar"));
        let Some(platform) = platform.filter(|platform| platform.is_file()) else {
            eprintln!("skipping: Android platform 35 is unavailable");
            return;
        };
        Self::success(
            Command::new("javac")
                .arg("-classpath")
                .arg(platform)
                .arg(&source),
        );
    }
}

#[test]
fn native_windows_controls_enforce_grants_bounds_and_thread_in_executed_host_code() {
    NativeWindowCheck::windows();
}

#[test]
fn native_android_controls_typecheck_with_the_platform_sdk() {
    NativeWindowCheck::android();
}
