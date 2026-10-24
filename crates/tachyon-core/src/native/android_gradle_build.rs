use super::android_gradle_cache::AndroidGradleCache;
use super::android_release::AndroidRelease;
use super::config::NativeApplication;
use super::host::{first_line, native_io, native_tool_failure, run_tool_in};
use super::native_build_folders::NativeBuildFolders;
use super::native_reproducibility::NativeReproducibility;
use crate::Failure;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct AndroidGradleBuild;

impl AndroidGradleBuild {
    const GRADLE_ARGUMENTS: [&'static str; 2] = ["--no-daemon", "--console=plain"];

    async fn run_gradle(
        project: &Path,
        tasks: &[&str],
        environment: &[(&str, &str)],
    ) -> Result<(), Failure> {
        let cache = AndroidGradleCache::from_environment(|name| std::env::var_os(name));
        let offline = cache.arguments(AndroidGradleCache::has_kotlin(project));
        let outcome = Self::gradle(project, [&offline[..], tasks], environment).await;
        let retry = outcome
            .as_ref()
            .err()
            .is_some_and(|failure| AndroidGradleCache::retry_online(failure, &offline));
        if !retry {
            return outcome;
        }
        Self::gradle(project, [&[], tasks], environment).await
    }

    async fn gradle(
        project: &Path,
        [offline, tasks]: [&[&str]; 2],
        environment: &[(&str, &str)],
    ) -> Result<(), Failure> {
        let arguments: Vec<&str> = Self::GRADLE_ARGUMENTS
            .iter()
            .chain(offline)
            .chain(tasks)
            .copied()
            .collect();
        NativeReproducibility::run_long_tool_in_with(
            "gradle",
            &arguments,
            Some(project),
            environment,
        )
        .await
        .map(|_| ())
    }

    async fn prepared_gradle(project: &Path) -> Result<String, Failure> {
        if std::env::var_os("ANDROID_HOME").is_none()
            && std::env::var_os("ANDROID_SDK_ROOT").is_none()
        {
            return Err(native_tool_failure(
                1605,
                "The Android host requires ANDROID_HOME or ANDROID_SDK_ROOT on the build machine.",
            ));
        }
        Ok(first_line(
            &run_tool_in("gradle", &["--version"], Some(project)).await?,
            "Gradle unknown",
        ))
    }

    pub(super) async fn debug(
        project: &Path,
        bundle: &Path,
        application: &NativeApplication,
    ) -> Result<(String, String), Failure> {
        let version = Self::prepared_gradle(project).await?;
        Self::run_gradle(project, &["assembleDebug"], &[]).await?;
        let built = project.join("app/build/outputs/apk/debug/app-debug.apk");
        let name = format!("{}.apk", application.executable_name);
        let published = bundle.join(&name);
        native_io(fs::copy(&built, &published), &published)?;
        NativeBuildFolders::remove(project, &NativeBuildFolders::GRADLE)?;
        Ok((version, name))
    }

    pub(super) async fn release(
        project: &Path,
        bundle: &Path,
        application: &NativeApplication,
    ) -> Result<(String, String), Failure> {
        let working_directory = std::env::current_dir().map_err(|error| {
            native_tool_failure(1605, &format!("Cannot read the working directory: {error}"))
        })?;
        let signing =
            AndroidRelease::from_environment(|name| std::env::var_os(name), &working_directory)?;
        let version = Self::prepared_gradle(project).await?;
        let environment = signing.gradle_environment();
        let environment: Vec<(&str, &str)> = environment
            .iter()
            .map(|(name, value)| (*name, value.as_str()))
            .collect();
        Self::run_gradle(project, &AndroidRelease::GRADLE_TASKS, &environment).await?;
        let name = format!("{}.apk", application.executable_name);
        let published_apk = bundle.join(&name);
        let built_apk = project.join(AndroidRelease::BUILT_APK);
        native_io(fs::copy(&built_apk, &published_apk), &published_apk)?;
        let published_bundle = bundle.join(format!("{}.aab", application.executable_name));
        let built_bundle = project.join(AndroidRelease::BUILT_BUNDLE);
        native_io(
            fs::copy(&built_bundle, &published_bundle),
            &published_bundle,
        )?;
        NativeBuildFolders::remove(project, &NativeBuildFolders::GRADLE)?;
        Ok((version, name))
    }
}
