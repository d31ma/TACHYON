use super::{CppFailure, CppFolder, CppRemote, CppService, CppStore};
use crate::Failure;
use serde_json::Value;
use std::path::Path;

pub(crate) struct CppContexts {
    setups: Vec<String>,
}

impl CppContexts {
    const SETUPS: &'static str = "client/setups";
    const NO_CONTEXT: &'static str = " ::tachyon::TacLazyContext tachyonBehaviors;";

    pub(crate) fn read(root: &Path) -> Result<Self, Failure> {
        let setups = CppFolder::files(root, Self::SETUPS)?
            .iter()
            .filter_map(|path| path.file_stem())
            .map(|stem| stem.to_string_lossy().into_owned())
            .collect();
        Ok(Self { setups })
    }

    pub(crate) fn member(
        &self,
        service: &CppService,
        entities: &[Value],
    ) -> Result<String, Failure> {
        let class = service.class();
        let remote = service
            .is_remote()
            .then(|| self.client(service))
            .transpose()?;
        let store = CppStore::setup(class)
            .map(|setup| self.store(service, entities, setup))
            .transpose()?;
        Ok([remote, store].into_iter().flatten().collect())
    }

    fn client(&self, service: &CppService) -> Result<String, Failure> {
        let class = service.class();
        let setup = CppRemote::setup(class);
        let found = self.setups.contains(&setup);
        let missing = [!found, CppRemote::names_setup(class)] == [true, true];
        (!missing)
            .then_some(())
            .ok_or_else(|| Self::no_setup(service, "HTTPClient", &setup))?;
        let opened = format!(
            " ::tachyon::TacLazyContext tachyonBehaviors = ::tachyon::TacLazyContext::of<::{setup}>({});",
            CppRemote::literal(&CppRemote::adapter(class))
        );
        Ok([Self::NO_CONTEXT.to_owned(), opened][usize::from(found)].clone())
    }

    fn store(
        &self,
        service: &CppService,
        entities: &[Value],
        (setup, partition, named): (String, String, bool),
    ) -> Result<String, Failure> {
        let entity =
            CppStore::kept_entity(&service.path, &service.source, service.class(), entities)?;
        let found = self.setups.contains(&setup);
        let built_in = entity["adapter"] == CppStore::BUILT_IN;
        let missing = [!found, [named, !built_in].contains(&true)] == [true, true];
        (!missing)
            .then_some(())
            .ok_or_else(|| Self::no_setup(service, "Store", &setup))?;
        let opened = format!(
            " ::tachyon::TacStoreContext tachyonStore{{{}, ::tachyon::TacLazyContext::of<::{setup}>({})}};",
            CppRemote::literal(entity["name"].as_str().unwrap_or_default()),
            CppRemote::literal(&partition)
        );
        Ok([String::new(), opened][usize::from(found)].clone())
    }

    fn no_setup(service: &CppService, role: &str, setup: &str) -> Failure {
        let class = service.class();
        CppFailure::unreadable(
            &service.path,
            class.name_at,
            &format!(
                "The C++ {role} {} opens its behavior context from the Setup {setup}, and there is no {}/{setup}.hpp. Write `class [[Setup]] {setup}` there with `static BehaviorAdapters configure(const BehaviorExecution& execution)`, or name another Setup with {{\"setup\":\"CatalogSetup\"}}. A Store whose Entity names an adapter other than \"indexeddb\" gets that adapter from its Setup",
                class.name,
                Self::SETUPS
            ),
        )
    }
}
