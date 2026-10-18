use super::{
    CppYonController, CppYonHandler, CppYonHooks, CppYonPolicies, CppYonRemote, CppYonScope,
    CppYonStores,
};
use crate::Failure;
use crate::tac_cpp::{CppBehaviors, CppFile, CppLifetime, CppLowering, CppProgram, CppService};
use crate::yon::plan::Scope;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(crate) struct CppYonProvider {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) at: usize,
    pub(crate) scope: Scope,
    pub(crate) needs: Vec<String>,
    pub(crate) hooks: CppYonHooks,
    pub(crate) lowered: String,
    pub(crate) entity: Option<String>,
}

impl CppYonProvider {
    const FRIENDS: &'static str =
        " friend class ::tachyon::yon::YonNeeds; friend class TachyonYonRoutes;";

    pub(crate) fn service(
        service: &CppService,
        models: &CppProgram,
        stores: &CppYonStores,
    ) -> Result<Self, Failure> {
        let written = (
            service.path.as_path(),
            &service.file,
            service.source.as_str(),
        );
        let remote = CppYonRemote::members(service);
        let mut provider = Self::read(
            written,
            &service.behaviors,
            (Scope::Singleton, &remote),
            models,
            stores,
        )?;
        provider
            .lowered
            .push_str(&CppYonRemote::definitions(service));
        Ok(provider)
    }

    pub(crate) fn root(
        controller: &CppYonController,
        models: &CppProgram,
        stores: &CppYonStores,
    ) -> Result<Self, Failure> {
        let written = (
            controller.path.as_path(),
            &controller.file,
            controller.source.as_str(),
        );
        Self::rooted(written, models, stores)
    }

    pub(crate) fn handler(
        handler: &CppYonHandler,
        models: &CppProgram,
        stores: &CppYonStores,
    ) -> Result<Self, Failure> {
        let written = (
            handler.path.as_path(),
            &handler.file,
            handler.source.as_str(),
        );
        Self::rooted(written, models, stores)
    }

    fn rooted(
        (path, file, source): (&Path, &CppFile, &str),
        models: &CppProgram,
        stores: &CppYonStores,
    ) -> Result<Self, Failure> {
        CppService::check_needs(path, &file.class)?;
        CppLifetime::check(path, &file.class)?;
        let behaviors = CppBehaviors::policies(path, source, &file.class)?;
        Self::read(
            (path, file, source),
            &behaviors,
            (Scope::Request, ""),
            models,
            stores,
        )
    }

    fn read(
        (path, file, source): (&Path, &CppFile, &str),
        behaviors: &BTreeMap<String, Value>,
        (unwritten, remote): (Scope, &str),
        models: &CppProgram,
        stores: &CppYonStores,
    ) -> Result<Self, Failure> {
        let class = &file.class;
        models.check_role(path, source, class)?;
        let scope = CppYonPolicies::scope(path, class, behaviors)?;
        let opening = format!(
            "{}{remote}{}",
            Self::FRIENDS,
            stores.member(path, source, class)?
        );
        let lowered = CppLowering::yon(file, source, &opening, behaviors, scope);
        let (entity, defined) = stores.definitions(path, source, class)?.unzip();
        Ok(Self {
            name: class.name.clone(),
            path: path.to_path_buf(),
            at: class.name_at,
            scope: CppYonScope::of(path, class, unwritten)?,
            needs: CppLifetime::injections(class)
                .into_iter()
                .map(|injection| injection.provider)
                .collect(),
            hooks: CppYonHooks::read(path, class)?,
            lowered: CppProgram::located(path, &lowered)
                + &models.conversions_of(path, class)
                + &defined.unwrap_or_default(),
            entity,
        })
    }
}
