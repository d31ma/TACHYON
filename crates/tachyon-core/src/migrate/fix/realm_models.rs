use super::known_model::KnownModel;
use crate::route_types::ModelTable;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Default)]
pub(super) struct RealmModels {
    known: BTreeMap<(String, String), KnownModel>,
}

impl RealmModels {
    const REALMS: [&'static str; 2] = ["client", "server"];
    const MODELS: &'static str = "models";
    const LANGUAGES: [&'static str; 2] = ["js", "ts"];

    pub(super) fn read(sources: &[(String, String)]) -> Self {
        let known = sources
            .iter()
            .flat_map(|(relative, source)| Self::file_models(relative, source))
            .collect();
        Self { known }
    }

    pub(super) fn find(&self, realm: &str, name: &str) -> Option<&KnownModel> {
        self.known.get(&(realm.to_owned(), name.to_owned()))
    }

    pub(super) fn add(&mut self, realm: &str, name: &str, known: KnownModel) {
        self.known
            .insert((realm.to_owned(), name.to_owned()), known);
    }

    fn file_models(relative: &str, source: &str) -> Vec<((String, String), KnownModel)> {
        let Some(realm) = Self::model_realm(relative) else {
            return Vec::new();
        };
        Self::models_in(realm, relative, source)
    }

    fn models_in(realm: &str, relative: &str, source: &str) -> Vec<((String, String), KnownModel)> {
        let declared = ModelTable::declared_models(Self::language(relative), source);
        declared
            .into_iter()
            .map(|(name, model)| {
                (
                    (realm.to_owned(), name),
                    KnownModel::existing(relative, model),
                )
            })
            .collect()
    }

    fn model_realm(relative: &str) -> Option<&str> {
        let mut parts = relative.split('/');
        let realm = parts.next().unwrap_or_default();
        let checks = [
            Self::REALMS.contains(&realm),
            parts.next() == Some(Self::MODELS),
            Self::LANGUAGES.contains(&Self::language(relative)),
        ];
        (!checks.contains(&false)).then_some(realm)
    }

    fn language(relative: &str) -> &str {
        let extension = Path::new(relative).extension();
        extension
            .and_then(|value| value.to_str())
            .unwrap_or_default()
    }
}
