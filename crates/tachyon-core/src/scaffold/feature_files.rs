use super::{AppFeature, ProjectChoices};

pub(crate) struct FeatureFiles;

impl FeatureFiles {
    const CLIENT: &'static str = "client/";
    const SERVER: &'static str = "server/";

    pub(crate) fn for_choices(choices: &ProjectChoices) -> Vec<(String, String)> {
        Self::for_features(&choices.features, choices)
    }

    pub(crate) fn for_features(
        features: &[AppFeature],
        choices: &ProjectChoices,
    ) -> Vec<(String, String)> {
        let mut files = Self::client_files(features, choices);
        files.extend(Self::server_files(features, choices));
        files
    }

    fn client_files(features: &[AppFeature], choices: &ProjectChoices) -> Vec<(String, String)> {
        if !choices.realm.has_client() {
            return Vec::new();
        }
        Self::files_under(features, Self::CLIENT, choices.client_uses_typescript())
    }

    fn server_files(features: &[AppFeature], choices: &ProjectChoices) -> Vec<(String, String)> {
        if !choices.realm.has_server() {
            return Vec::new();
        }
        Self::files_under(features, Self::SERVER, choices.server_uses_typescript())
    }

    fn files_under(
        features: &[AppFeature],
        realm: &str,
        typescript: bool,
    ) -> Vec<(String, String)> {
        features
            .iter()
            .flat_map(|feature| feature.files(typescript))
            .filter(|(path, _)| path.starts_with(realm))
            .collect()
    }
}
