use crate::Failure;
use crate::failure::diagnostic;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AndroidRelease {
    keystore: PathBuf,
}

impl AndroidRelease {
    pub(super) const KEYSTORE: &'static str = "TAC_ANDROID_KEYSTORE";
    pub(super) const KEYSTORE_PASSWORD: &'static str = "TAC_ANDROID_KEYSTORE_PASSWORD";
    pub(super) const KEY_ALIAS: &'static str = "TAC_ANDROID_KEYSTORE_ALIAS";
    pub(super) const KEY_PASSWORD: &'static str = "TAC_ANDROID_KEYSTORE_KEY_PASSWORD";
    pub(super) const GRADLE_TASKS: [&'static str; 2] = ["assembleRelease", "bundleRelease"];
    pub(super) const BUILT_APK: &'static str = "app/build/outputs/apk/release/app-release.apk";
    pub(super) const BUILT_BUNDLE: &'static str =
        "app/build/outputs/bundle/release/app-release.aab";
    const REQUIRED: [&'static str; 3] = [Self::KEYSTORE, Self::KEYSTORE_PASSWORD, Self::KEY_ALIAS];
    const HELP: &'static str = "Set TAC_ANDROID_KEYSTORE to the keystore file, \
         TAC_ANDROID_KEYSTORE_PASSWORD to its password and TAC_ANDROID_KEYSTORE_ALIAS to the key's \
         alias. Set TAC_ANDROID_KEYSTORE_KEY_PASSWORD only when the key has its own password. \
         To build a debug APK instead, remove --release.";

    pub(super) fn from_environment(
        lookup: impl Fn(&str) -> Option<OsString>,
        working_directory: &Path,
    ) -> Result<Self, Failure> {
        Self::require_variables(&lookup)?;
        let named = PathBuf::from(lookup(Self::KEYSTORE).unwrap_or_default());
        let keystore = working_directory.join(named);
        Self::require_file(&keystore)?;
        Ok(Self { keystore })
    }

    fn require_variables(lookup: &impl Fn(&str) -> Option<OsString>) -> Result<(), Failure> {
        let missing: Vec<&str> = Self::REQUIRED
            .into_iter()
            .filter(|name| lookup(name).is_none_or(|value| value.is_empty()))
            .collect();
        missing.is_empty().then_some(()).ok_or_else(|| {
            Self::failure(&format!(
                "A release Android build is signed with a keystore from the environment, but \
                 these variables are missing: {}.",
                missing.join(", ")
            ))
        })
    }

    fn require_file(keystore: &Path) -> Result<(), Failure> {
        keystore.is_file().then_some(()).ok_or_else(|| {
            Self::failure(&format!(
                "TAC_ANDROID_KEYSTORE names '{}', which is not a keystore file.",
                keystore.display()
            ))
        })
    }

    pub(super) fn gradle_environment(&self) -> Vec<(&'static str, String)> {
        vec![(Self::KEYSTORE, self.keystore.display().to_string())]
    }

    pub(super) fn gradle_signing_config() -> String {
        format!(
            r#"  signingConfigs {{
    create("release") {{
      System.getenv("{keystore}")?.let {{ storeFile = file(it) }}
      storePassword = System.getenv("{keystore_password}")
      keyAlias = System.getenv("{key_alias}")
      keyPassword = System.getenv("{key_password}") ?: System.getenv("{keystore_password}")
    }}
  }}
"#,
            keystore = Self::KEYSTORE,
            keystore_password = Self::KEYSTORE_PASSWORD,
            key_alias = Self::KEY_ALIAS,
            key_password = Self::KEY_PASSWORD,
        )
    }

    fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(
            1605,
            message,
            Some(String::from(Self::HELP)),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::AndroidRelease;
    use std::collections::BTreeMap;
    use std::ffi::OsString;

    fn lookup<'values>(
        values: &'values BTreeMap<&'static str, String>,
    ) -> impl Fn(&str) -> Option<OsString> + 'values {
        |name| values.get(name).map(OsString::from)
    }

    #[test]
    fn a_release_build_names_every_missing_signing_variable() {
        let root = tempfile::tempdir().expect("root");
        let mut values = BTreeMap::new();
        values.insert(AndroidRelease::KEY_ALIAS, String::from("upload"));
        values.insert(AndroidRelease::KEYSTORE_PASSWORD, String::new());
        let failure = AndroidRelease::from_environment(lookup(&values), root.path())
            .expect_err("missing keystore");
        let text = failure.to_string();
        assert!(text.contains("TY1605"));
        assert!(text.contains("missing: TAC_ANDROID_KEYSTORE, TAC_ANDROID_KEYSTORE_PASSWORD."));
        assert!(!text.contains("upload"));
    }

    #[test]
    fn a_keystore_that_is_not_a_file_is_refused_before_gradle_runs() {
        let root = tempfile::tempdir().expect("root");
        let mut values = BTreeMap::new();
        values.insert(AndroidRelease::KEYSTORE, String::from("missing.jks"));
        values.insert(
            AndroidRelease::KEYSTORE_PASSWORD,
            String::from("secret-value"),
        );
        values.insert(AndroidRelease::KEY_ALIAS, String::from("upload"));
        let failure = AndroidRelease::from_environment(lookup(&values), root.path())
            .expect_err("missing file");
        let text = failure.to_string();
        assert!(text.contains("missing.jks"));
        assert!(!text.contains("secret-value"));
    }

    #[test]
    fn a_relative_keystore_reaches_gradle_as_a_path_from_the_working_directory() {
        let root = tempfile::tempdir().expect("root");
        std::fs::create_dir_all(root.path().join("keys")).expect("keys");
        std::fs::write(root.path().join("keys/upload.jks"), b"keystore").expect("keystore");
        let mut values = BTreeMap::new();
        values.insert(AndroidRelease::KEYSTORE, String::from("keys/upload.jks"));
        values.insert(
            AndroidRelease::KEYSTORE_PASSWORD,
            String::from("secret-value"),
        );
        values.insert(AndroidRelease::KEY_ALIAS, String::from("upload"));
        let release =
            AndroidRelease::from_environment(lookup(&values), root.path()).expect("release");
        let expected = root.path().join("keys/upload.jks").display().to_string();
        assert_eq!(
            release.gradle_environment(),
            vec![(AndroidRelease::KEYSTORE, expected)]
        );
    }

    #[test]
    fn the_gradle_signing_config_reads_only_environment_variable_names() {
        let gradle = AndroidRelease::gradle_signing_config();
        assert!(gradle.contains("create(\"release\")"));
        for name in [
            AndroidRelease::KEYSTORE,
            AndroidRelease::KEYSTORE_PASSWORD,
            AndroidRelease::KEY_ALIAS,
            AndroidRelease::KEY_PASSWORD,
        ] {
            assert!(gradle.contains(&format!("System.getenv(\"{name}\")")));
        }
        assert!(gradle.contains("?: System.getenv(\"TAC_ANDROID_KEYSTORE_PASSWORD\")"));
    }
}
