#![allow(clippy::expect_used)]

use super::AndroidNdk;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

struct AndroidNdkTests;

impl AndroidNdkTests {
    fn install(root: &Path, levels: &[u32]) -> PathBuf {
        let bin = root
            .join(AndroidNdk::PREBUILT)
            .join(AndroidNdk::host_tag())
            .join("bin");
        std::fs::create_dir_all(&bin).expect("bin");
        for level in levels {
            let wrapper = bin.join(AndroidNdk::wrapper_name("aarch64-linux-android", *level));
            std::fs::write(wrapper, "").expect("wrapper");
        }
        bin
    }

    fn environment(pairs: &[(&'static str, PathBuf)]) -> impl Fn(&str) -> Option<OsString> {
        let pairs = pairs.to_vec();
        move |name| {
            pairs
                .iter()
                .find(|(known, _)| *known == name)
                .map(|(_, value)| value.clone().into_os_string())
        }
    }
}

#[test]
fn the_newest_side_by_side_ndk_in_the_sdk_is_used() {
    let sdk = tempfile::tempdir().expect("sdk");
    AndroidNdkTests::install(&sdk.path().join("ndk/9.1.0"), &[26]);
    let newest = AndroidNdkTests::install(&sdk.path().join("ndk/30.0.16248370"), &[26]);
    let ndk = AndroidNdk::from_environment(AndroidNdkTests::environment(&[(
        "ANDROID_HOME",
        sdk.path().to_path_buf(),
    )]))
    .expect("ndk");
    let linker = ndk.linker("aarch64-linux-android", 26).expect("linker");
    assert_eq!(linker.parent(), Some(newest.as_path()));
}

#[test]
fn a_declared_ndk_wins_over_the_sdk_folder() {
    let sdk = tempfile::tempdir().expect("sdk");
    AndroidNdkTests::install(&sdk.path().join("ndk/30.0.1"), &[26]);
    let declared = tempfile::tempdir().expect("declared");
    let bin = AndroidNdkTests::install(declared.path(), &[26]);
    let ndk = AndroidNdk::from_environment(AndroidNdkTests::environment(&[
        ("ANDROID_HOME", sdk.path().to_path_buf()),
        ("ANDROID_NDK_HOME", declared.path().to_path_buf()),
    ]))
    .expect("ndk");
    assert_eq!(
        ndk.linker("aarch64-linux-android", 26)
            .expect("linker")
            .parent(),
        Some(bin.as_path())
    );
}

#[test]
fn the_linker_is_the_newest_api_level_at_or_below_the_app_floor() {
    let root = tempfile::tempdir().expect("ndk");
    AndroidNdkTests::install(root.path(), &[21, 24, 35]);
    let ndk = AndroidNdk::from_environment(AndroidNdkTests::environment(&[(
        "ANDROID_NDK_HOME",
        root.path().to_path_buf(),
    )]))
    .expect("ndk");
    let name = |level| {
        ndk.linker("aarch64-linux-android", level)
            .expect("linker")
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    };
    assert_eq!(
        name(26),
        Some(AndroidNdk::wrapper_name("aarch64-linux-android", 24))
    );
    assert_eq!(
        name(40),
        Some(AndroidNdk::wrapper_name("aarch64-linux-android", 35))
    );
    let missing = ndk
        .linker("x86_64-linux-android", 26)
        .expect_err("no x86_64 clang");
    assert!(format!("{missing:?}").contains("x86_64-linux-android at API level 26"));
}

#[test]
fn a_missing_ndk_says_how_to_install_it() {
    let empty = tempfile::tempdir().expect("sdk");
    let failure = AndroidNdk::from_environment(AndroidNdkTests::environment(&[(
        "ANDROID_HOME",
        empty.path().to_path_buf(),
    )]))
    .expect_err("no ndk");
    let text = format!("{failure:?}");
    assert!(text.contains("sdkmanager"), "{text}");
    assert!(text.contains("ANDROID_NDK_HOME"), "{text}");
}
