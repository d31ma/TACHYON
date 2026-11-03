#![allow(clippy::expect_used)]

use super::AndroidGradleCache;
use crate::failure::diagnostic;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

struct AndroidGradleCacheTests;

impl AndroidGradleCacheTests {
    fn cache(user_home: &Path) -> AndroidGradleCache {
        let values = BTreeMap::from([("GRADLE_USER_HOME", user_home.as_os_str().to_owned())]);
        AndroidGradleCache::from_environment(|name| values.get(name).cloned())
    }

    fn fill(user_home: &Path, modules: &[String]) {
        for module in modules {
            let folder = user_home
                .join("caches/modules-2/files-2.1")
                .join(module.replace(':', "/"));
            std::fs::create_dir_all(folder).expect("module folder");
        }
    }

    fn failure(message: &str) -> crate::Failure {
        crate::Failure::one(diagnostic(1605, message, None, None))
    }
}

#[test]
fn the_gradle_cache_is_under_gradle_user_home_or_the_home_folder() {
    let named = AndroidGradleCache::from_environment(|name| {
        (name == "GRADLE_USER_HOME").then(|| OsString::from("/cache/gradle"))
    });
    assert_eq!(
        named.modules,
        Path::new("/cache/gradle/caches/modules-2/files-2.1")
    );
    let home = AndroidGradleCache::from_environment(|name| {
        (name == "USERPROFILE").then(|| OsString::from("/users/ada"))
    });
    assert_eq!(
        home.modules,
        Path::new("/users/ada/.gradle/caches/modules-2/files-2.1")
    );
}

#[test]
fn gradle_runs_offline_only_when_the_cache_holds_every_pinned_module() {
    let user_home = tempfile::tempdir().expect("gradle home");
    let cache = AndroidGradleCacheTests::cache(user_home.path());
    assert!(cache.arguments(false).is_empty());
    let note = cache.note(false).expect("a note while the cache is empty");
    assert!(
        note.contains("com.android.tools.build:gradle:8.7.3"),
        "{note}"
    );
    assert!(note.contains("androidx.webkit:webkit:"), "{note}");
    assert!(!note.contains("kotlin"), "{note}");
    assert!(note.contains("--offline"), "{note}");

    AndroidGradleCacheTests::fill(user_home.path(), &AndroidGradleCache::pinned(false));
    assert_eq!(cache.arguments(false), [AndroidGradleCache::OFFLINE]);
    assert_eq!(cache.note(false), None);
    assert!(cache.arguments(true).is_empty());
    let kotlin = cache.note(true).expect("the Kotlin plugin is not cached");
    assert!(
        kotlin.contains("org.jetbrains.kotlin:kotlin-gradle-plugin:"),
        "{kotlin}"
    );

    AndroidGradleCacheTests::fill(user_home.path(), &AndroidGradleCache::pinned(true));
    assert_eq!(cache.arguments(true), [AndroidGradleCache::OFFLINE]);
}

#[test]
fn an_offline_build_that_misses_a_module_runs_again_online() {
    let offline = [AndroidGradleCache::OFFLINE];
    let refused = AndroidGradleCacheTests::failure(
        "Could not resolve com.android.tools.build:aapt2:8.7.3-12006047. No cached version of \
         com.android.tools.build:aapt2:8.7.3-12006047 available for offline mode.",
    );
    assert!(AndroidGradleCache::retry_online(&refused, &offline));
    assert!(!AndroidGradleCache::retry_online(&refused, &[]));
    let plugin = AndroidGradleCacheTests::failure(
        "Plugin [id: 'org.jetbrains.kotlin.android', version: '2.0.21'] was not found in any of \
         the following sources: Plugin Repositories (could not resolve plugin artifact \
         'org.jetbrains.kotlin.android:org.jetbrains.kotlin.android.gradle.plugin:2.0.21')",
    );
    assert!(AndroidGradleCache::retry_online(&plugin, &offline));
    let broken = AndroidGradleCacheTests::failure("Compilation failed; see the compiler output.");
    assert!(!AndroidGradleCache::retry_online(&broken, &offline));
}
