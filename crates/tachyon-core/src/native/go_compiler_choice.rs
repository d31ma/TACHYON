use super::go_compiler::GoCompiler;
use super::host::native_tool_failure;
use crate::Failure;
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct GoCompilerChoice {
    pub(super) first: GoCompiler,
    pub(super) fallback: Option<GoCompiler>,
    pub(super) reason: &'static str,
}

impl GoCompilerChoice {
    const MANIFEST: &'static str = "client/manifest.json";
    const SETTING: &'static str = "/tac/go/compiler";
    const TINYGO_SETTING: &'static str = "tinygo";
    const STANDARD_SETTING: &'static str = "go";
    const FAILURE_CODE: u16 = 1605;
    const SMALLEST: Self = Self {
        first: GoCompiler::TINYGO,
        fallback: Some(GoCompiler::STANDARD),
        reason: "the smallest download",
    };
    const FASTEST: Self = Self {
        first: GoCompiler::STANDARD,
        fallback: None,
        reason: "the fastest development build",
    };
    const ONLY_TINYGO: Self = Self {
        first: GoCompiler::TINYGO,
        fallback: None,
        reason: "tac.go.compiler is \"tinygo\"",
    };
    const ONLY_STANDARD: Self = Self {
        first: GoCompiler::STANDARD,
        fallback: None,
        reason: "tac.go.compiler is \"go\"",
    };

    pub(super) fn read(
        snapshot_root: &Path,
        profile: crate::BuildProfile,
    ) -> Result<Self, Failure> {
        let setting = Self::setting(snapshot_root);
        Self::for_setting(setting.as_deref(), profile)
    }

    fn setting(snapshot_root: &Path) -> Option<String> {
        let bytes = std::fs::read(snapshot_root.join(Self::MANIFEST)).ok()?;
        let manifest: Value = serde_json::from_slice(&bytes).ok()?;
        manifest.pointer(Self::SETTING)?.as_str().map(String::from)
    }

    fn for_setting(setting: Option<&str>, profile: crate::BuildProfile) -> Result<Self, Failure> {
        match (setting, profile) {
            (Some(Self::TINYGO_SETTING), _) => Ok(Self::ONLY_TINYGO),
            (Some(Self::STANDARD_SETTING), _) => Ok(Self::ONLY_STANDARD),
            (Some(other), _) => Err(Self::unknown(other)),
            (None, crate::BuildProfile::Release) => Ok(Self::SMALLEST),
            (None, crate::BuildProfile::Development) => Ok(Self::FASTEST),
        }
    }

    fn unknown(setting: &str) -> Failure {
        native_tool_failure(
            Self::FAILURE_CODE,
            &format!(
                "tac.go.compiler in client/manifest.json is \"{setting}\"; use \"tinygo\" or \"go\", or remove it to let release builds try TinyGo first"
            ),
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{GoCompiler, GoCompilerChoice};
    use crate::BuildProfile;

    #[test]
    fn a_release_build_tries_tinygo_first_and_a_development_build_uses_go() {
        let release = GoCompilerChoice::for_setting(None, BuildProfile::Release).expect("choice");
        assert_eq!(release.first, GoCompiler::TINYGO);
        assert_eq!(release.fallback, Some(GoCompiler::STANDARD));
        let development =
            GoCompilerChoice::for_setting(None, BuildProfile::Development).expect("choice");
        assert_eq!(development.first, GoCompiler::STANDARD);
        assert_eq!(development.fallback, None);
    }

    #[test]
    fn the_setting_forces_one_compiler_without_a_fallback() {
        let tinygo = GoCompilerChoice::for_setting(Some("tinygo"), BuildProfile::Development)
            .expect("choice");
        assert_eq!((tinygo.first, tinygo.fallback), (GoCompiler::TINYGO, None));
        let standard =
            GoCompilerChoice::for_setting(Some("go"), BuildProfile::Release).expect("choice");
        assert_eq!(
            (standard.first, standard.fallback),
            (GoCompiler::STANDARD, None)
        );
    }

    #[test]
    fn an_unknown_setting_names_the_allowed_values() {
        let failure = GoCompilerChoice::for_setting(Some("gccgo"), BuildProfile::Release)
            .expect_err("unknown compiler");
        assert!(
            failure.to_string().contains("\"tinygo\" or \"go\""),
            "{failure}"
        );
    }
}
