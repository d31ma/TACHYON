#![allow(clippy::expect_used)]

use super::config::WindowConfiguration;
use super::native_window_controls::NativeWindowControls;
use std::process::Command;

struct NativeWindowCheck;

impl NativeWindowCheck {
    const MSVC_UNUSED_PARAMETER: [&str; 2] = ["-Wunused-parameter", "-Werror"];

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
        if !Self::compiler_available() {
            eprintln!("skipping: C compiler is unavailable");
            return;
        }
        let root = tempfile::tempdir().expect("fixture");
        let source = root.path().join("host.c");
        std::fs::write(&source, Self::windows_host(&Self::window())).expect("source");
        let program = root.path().join("host");
        Self::success(
            Command::new("cc")
                .args(Self::MSVC_UNUSED_PARAMETER)
                .arg(&source)
                .arg("-o")
                .arg(&program),
        );
        Self::success(&mut Command::new(&program));
    }

    fn windows_without_grants() {
        if !Self::compiler_available() {
            eprintln!("skipping: C compiler is unavailable");
            return;
        }
        let root = tempfile::tempdir().expect("fixture");
        let ungranted = root.path().join("ungranted.c");
        std::fs::write(
            &ungranted,
            Self::windows_host(&WindowConfiguration::default()),
        )
        .expect("source");
        Self::success(
            Command::new("cc")
                .args(Self::MSVC_UNUSED_PARAMETER)
                .arg("-fsyntax-only")
                .arg(&ungranted),
        );
    }

    fn compiler_available() -> bool {
        Command::new("cc")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn local_json() -> String {
        let json = include_str!("local_bundle.h")
            .split("static size_t tachyon_skip_space")
            .nth(1)
            .expect("JSON field parser");
        format!("static size_t tachyon_skip_space{json}")
    }

    fn windows_host(window: &WindowConfiguration) -> String {
        include_str!("../../tests/fixtures/native/windows_capability_host.c")
            .replace("__LOCAL_JSON__", &Self::local_json())
            .replace("__WINDOW_HOST__", &NativeWindowControls::windows(window))
    }

    #[cfg(windows)]
    fn win32() {
        let root = tempfile::tempdir().expect("fixture");
        let source = root.path().join("hwnd.c");
        let host = include_str!("../../tests/fixtures/native/windows_window_controls_hwnd.c")
            .replace("__LOCAL_JSON__", &Self::local_json())
            .replace(
                "__WINDOW_HOST__",
                &NativeWindowControls::windows(&Self::window()),
            );
        std::fs::write(&source, host).expect("source");
        let program = root.path().join("hwnd.exe");
        Self::success(
            Command::new("cl")
                .current_dir(root.path())
                .args(["/nologo", "/W3", "/WX"])
                .arg(&source)
                .arg(format!("/Fe:{}", program.display()))
                .arg("user32.lib"),
        );
        let output = Command::new(&program).output().expect("win32 check");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        println!("{}", stdout.trim());
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
fn native_windows_controls_without_grants_leave_no_unused_parameter_for_msvc() {
    NativeWindowCheck::windows_without_grants();
}

#[cfg(windows)]
#[test]
#[ignore = "Requires Windows and MSVC cl on PATH (vcvars); drives a real Win32 window, not WebView2"]
fn native_windows_controls_drive_a_real_win32_window() {
    NativeWindowCheck::win32();
}

#[test]
fn native_android_controls_typecheck_with_the_platform_sdk() {
    NativeWindowCheck::android();
}
