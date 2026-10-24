use super::android::AndroidHostGenerator;
use crate::Failure;
use crate::project::NativeCompanion;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tachyon_contracts::NativeTarget;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AndroidGradleCache {
    modules: PathBuf,
}

impl AndroidGradleCache {
    pub(super) const OFFLINE: &'static str = "--offline";
    const OFFLINE_REFUSALS: [&'static str; 2] = ["offline mode", "could not resolve"];
    const USER_HOME: &'static str = "GRADLE_USER_HOME";
    const HOME_VARIABLES: [&'static str; 2] = ["HOME", "USERPROFILE"];
    const GRADLE_FOLDER: &'static str = ".gradle";
    const MODULES: &'static str = "caches/modules-2/files-2.1";
    const KOTLIN_FOLDER: &'static str = "app/src/main/kotlin";

    pub(super) fn from_environment(variable: impl Fn(&str) -> Option<OsString>) -> Self {
        let user_home = variable(Self::USER_HOME).map(PathBuf::from).or_else(|| {
            Self::HOME_VARIABLES
                .iter()
                .find_map(|name| variable(name))
                .map(|home| PathBuf::from(home).join(Self::GRADLE_FOLDER))
        });
        Self {
            modules: user_home.unwrap_or_default().join(Self::MODULES),
        }
    }

    pub(super) fn pinned(has_kotlin: bool) -> Vec<String> {
        let plugin = AndroidHostGenerator::ANDROID_GRADLE_PLUGIN;
        let kotlin = AndroidHostGenerator::KOTLIN_PLUGIN;
        let mut modules = vec![
            format!("com.android.tools.build:gradle:{plugin}"),
            format!("com.android.application:com.android.application.gradle.plugin:{plugin}"),
            format!(
                "androidx.webkit:webkit:{}",
                AndroidHostGenerator::ANDROIDX_WEBKIT
            ),
        ];
        modules.extend(
            has_kotlin
                .then(|| {
                    [
                        format!("org.jetbrains.kotlin:kotlin-gradle-plugin:{kotlin}"),
                        format!(
                            "org.jetbrains.kotlin.android:org.jetbrains.kotlin.android.gradle.plugin:{kotlin}"
                        ),
                    ]
                })
                .into_iter()
                .flatten(),
        );
        modules
    }

    pub(super) fn missing(&self, has_kotlin: bool) -> Vec<String> {
        Self::pinned(has_kotlin)
            .into_iter()
            .filter(|module| !self.holds(module))
            .collect()
    }

    fn holds(&self, module: &str) -> bool {
        self.modules.join(module.replace(':', "/")).is_dir()
    }

    pub(super) fn arguments(&self, has_kotlin: bool) -> Vec<&'static str> {
        self.missing(has_kotlin)
            .is_empty()
            .then_some(Self::OFFLINE)
            .into_iter()
            .collect()
    }

    pub(super) fn note(&self, has_kotlin: bool) -> Option<String> {
        let missing = self.missing(has_kotlin);
        (!missing.is_empty()).then(|| {
            format!(
                "Note: Gradle downloads {} for this Android build, because its cache in {} does \
                 not hold them yet. Later Android builds run Gradle with --offline once it does.",
                missing.join(", "),
                self.modules.display()
            )
        })
    }

    pub(super) fn build_note(
        target: NativeTarget,
        package: bool,
        companions: &[super::registry::NativeCompanionInput],
    ) -> Option<String> {
        let has_kotlin = companions
            .iter()
            .any(|companion| companion.language == NativeCompanion::Kotlin);
        let gradle_runs = [target == NativeTarget::Android, package] == [true, true];
        gradle_runs
            .then(|| Self::from_environment(|name| std::env::var_os(name)).note(has_kotlin))
            .flatten()
    }

    pub(super) fn has_kotlin(project: &Path) -> bool {
        project.join(Self::KOTLIN_FOLDER).is_dir()
    }

    pub(super) fn retry_online(failure: &Failure, arguments: &[&str]) -> bool {
        let offline = arguments.contains(&Self::OFFLINE);
        let text = failure.to_string().to_lowercase();
        let refused = Self::OFFLINE_REFUSALS
            .iter()
            .any(|refusal| text.contains(refusal));
        [offline, refused] == [true, true]
    }
}
