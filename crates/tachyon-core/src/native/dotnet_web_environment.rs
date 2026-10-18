use std::ffi::{OsStr, OsString};

pub(super) struct DotnetWebEnvironment;

impl DotnetWebEnvironment {
    const EMSCRIPTEN_CONFIG: &'static str = "EM_CONFIG";
    const EMSDK_PREFIX: &'static str = "EMSDK";

    pub(super) fn without_emscripten_settings() -> Vec<(OsString, OsString)> {
        Self::kept_variables(std::env::vars_os())
    }

    fn kept_variables(
        variables: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Vec<(OsString, OsString)> {
        variables
            .into_iter()
            .filter(|(name, _)| !Self::is_emscripten_setting(name))
            .collect()
    }

    fn is_emscripten_setting(name: &OsStr) -> bool {
        let name = name.to_string_lossy().to_ascii_uppercase();
        let is_config = name == Self::EMSCRIPTEN_CONFIG;
        let is_emsdk = name.starts_with(Self::EMSDK_PREFIX);
        [is_config, is_emsdk].contains(&true)
    }
}

#[cfg(test)]
mod tests {
    use super::DotnetWebEnvironment;
    use std::ffi::OsString;

    fn variables(names: &[&str]) -> Vec<(OsString, OsString)> {
        names
            .iter()
            .map(|name| (OsString::from(name), OsString::from("value")))
            .collect()
    }

    #[test]
    fn the_emscripten_config_and_every_emsdk_variable_are_removed() {
        let environment = variables(&[
            "PATH",
            "EM_CONFIG",
            "EMSDK",
            "EMSDK_NODE",
            "EMSDK_PYTHON",
            "HOME",
            "DOTNET_ROOT",
        ]);

        let kept = DotnetWebEnvironment::kept_variables(environment);

        assert_eq!(kept, variables(&["PATH", "HOME", "DOTNET_ROOT"]));
    }

    #[test]
    fn names_match_without_case_because_windows_ignores_it() {
        let environment = variables(&["Em_Config", "emsdk", "Path"]);

        let kept = DotnetWebEnvironment::kept_variables(environment);

        assert_eq!(kept, variables(&["Path"]));
    }

    #[test]
    fn other_emscripten_settings_and_similar_names_stay() {
        let environment = variables(&["EM_CACHE", "EM_CONFIG_BACKUP", "MY_EMSDK", "PATH"]);

        let kept = DotnetWebEnvironment::kept_variables(environment.clone());

        assert_eq!(kept, environment);
    }
}
