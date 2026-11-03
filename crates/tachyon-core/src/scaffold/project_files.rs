use super::{BaseFiles, ClientFiles, FeatureFiles, ProjectChoices, ServerFiles};
use std::collections::BTreeMap;

pub(crate) struct ProjectFiles;

impl ProjectFiles {
    pub(crate) fn for_choices(
        app_name: &str,
        choices: &ProjectChoices,
    ) -> BTreeMap<String, String> {
        let mut files = BTreeMap::new();
        files.extend(BaseFiles::for_choices(app_name, choices));
        files.extend(Self::client_files(app_name, choices));
        files.extend(Self::server_files(choices));
        files.extend(FeatureFiles::for_choices(choices));
        files
    }

    fn client_files(app_name: &str, choices: &ProjectChoices) -> Vec<(String, String)> {
        if !choices.realm.has_client() {
            return Vec::new();
        }
        ClientFiles::for_choices(app_name, choices)
    }

    fn server_files(choices: &ProjectChoices) -> Vec<(String, String)> {
        if !choices.realm.has_server() {
            return Vec::new();
        }
        ServerFiles::for_choices(choices)
    }
}
