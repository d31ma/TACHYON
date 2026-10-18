use super::CppYonController;
use crate::Failure;
use crate::behavior::{WrittenMarker, WrittenPolicies};
use crate::tac_cpp::{CppClass, CppFailure, CppFolder, CppProgram, CppSource, CppStore};
use serde_json::Value;
use std::path::Path;

pub(crate) struct CppYonStores {
    setups: Vec<String>,
    entities: Vec<Value>,
}

impl CppYonStores {
    const SETUPS: &'static str = "server/setups";
    const STORE: &'static str = "Store";

    pub(crate) fn read(root: &Path, models: &CppProgram) -> Result<Self, Failure> {
        let setups = CppFolder::files(root, Self::SETUPS)?
            .iter()
            .filter_map(|path| path.file_stem())
            .map(|stem| stem.to_string_lossy().into_owned())
            .collect();
        Ok(Self {
            setups,
            entities: models.entities(),
        })
    }

    pub(crate) fn member(
        &self,
        path: &Path,
        source: &str,
        class: &CppClass,
    ) -> Result<String, Failure> {
        class
            .markers
            .iter()
            .find(|marker| marker.name == Self::STORE)
            .map(|marker| self.opened(path, source, class, &marker.arguments, marker.at))
            .transpose()
            .map(Option::unwrap_or_default)
    }

    pub(crate) fn definitions(
        &self,
        path: &Path,
        source: &str,
        class: &CppClass,
    ) -> Result<Option<(String, String)>, Failure> {
        let defined = CppStore::definitions(path, source, class, &self.entities)?;
        Ok(defined.map(|(entity, text)| (entity, format!("{}{text}", CppProgram::GENERATED))))
    }

    fn opened(
        &self,
        path: &Path,
        source: &str,
        class: &CppClass,
        arguments: &str,
        at: usize,
    ) -> Result<String, Failure> {
        let written = WrittenPolicies::new(path, source, CppSource::EXTENSION);
        let store = written.store(&WrittenMarker {
            name: Self::STORE,
            arguments,
            at,
        })?;
        let (setup, partition, _) = CppStore::setup(class).unwrap_or_default();
        self.setups
            .contains(&setup)
            .then_some(())
            .ok_or_else(|| Self::no_setup(path, at, &class.name, &setup))?;
        Ok(format!(
            " ::tachyon::yon::YonStore<::{setup}> tachyonStore{{\"{}\", \"{}\"}};",
            store["entity"].as_str().unwrap_or_default(),
            partition
        ))
    }

    fn no_setup(path: &Path, at: usize, store: &str, setup: &str) -> Failure {
        CppFailure::coded(
            CppYonController::CODE,
            path,
            at,
            &format!(
                "The Yon C++ Store {store} opens its behavior context from the Setup {setup}, and there is no server/setups/{setup}.hpp. Write that Setup, whose configure gives the entity adapter, or name another one with {{\"setup\":\"CatalogSetup\"}} in the Store options"
            ),
        )
    }
}
