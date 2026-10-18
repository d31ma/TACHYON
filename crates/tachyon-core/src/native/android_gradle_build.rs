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
        NativeReproducibility::run_tool_in(
            "gradle",
            &["--no-daemon", "--console=plain", "assembleDebug"],
            Some(project),
        )
        .await?;
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
        let [assemble, package] = AndroidRelease::GRADLE_TASKS;
        NativeReproducibility::run_tool_in_with(
            "gradle",
            &["--no-daemon", "--console=plain", assemble, package],
            Some(project),
            &environment,
        )
        .await?;
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
