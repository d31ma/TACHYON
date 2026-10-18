use super::{CppYonController, CppYonHandler, CppYonHooks, CppYonScope};
use crate::Failure;
use crate::tac_cpp::{CppFile, CppLifetime, CppLowering, CppProgram, CppService};
use crate::yon::plan::Scope;
use std::path::{Path, PathBuf};

pub(crate) struct CppYonProvider {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) at: usize,
    pub(crate) scope: Scope,
    pub(crate) needs: Vec<String>,
    pub(crate) hooks: CppYonHooks,
    pub(crate) lowered: String,
}

impl CppYonProvider {
    const FRIENDS: &'static str =
        " friend class ::tachyon::yon::YonNeeds; friend class TachyonYonRoutes;";

    pub(crate) fn service(service: &CppService, models: &CppProgram) -> Result<Self, Failure> {
        Self::read(
            &service.path,
            &service.file,
            &service.source,
            Scope::Singleton,
            models,
        )
    }

    pub(crate) fn root(
        controller: &CppYonController,
        models: &CppProgram,
    ) -> Result<Self, Failure> {
        Self::rooted(
            &controller.path,
            &controller.file,
            &controller.source,
            models,
        )
    }

    pub(crate) fn handler(handler: &CppYonHandler, models: &CppProgram) -> Result<Self, Failure> {
        Self::rooted(&handler.path, &handler.file, &handler.source, models)
    }

    fn rooted(
        path: &Path,
        file: &CppFile,
        source: &str,
        models: &CppProgram,
    ) -> Result<Self, Failure> {
        CppService::check_needs(path, &file.class)?;
        CppLifetime::check(path, &file.class)?;
        Self::read(path, file, source, Scope::Request, models)
    }

    fn read(
        path: &Path,
        file: &CppFile,
        source: &str,
        unwritten: Scope,
        models: &CppProgram,
    ) -> Result<Self, Failure> {
        let class = &file.class;
        models.check_role(path, source, class)?;
        let lowered = CppLowering::yon(file, source, Self::FRIENDS);
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
            lowered: CppProgram::located(path, &lowered) + &models.conversions_of(path, class),
        })
    }
}
