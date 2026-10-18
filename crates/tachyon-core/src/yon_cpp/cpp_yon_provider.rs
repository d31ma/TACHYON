use super::{CppYonHooks, CppYonScope};
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

    pub(crate) fn service(service: &CppService) -> Result<Self, Failure> {
        Self::read(
            &service.path,
            &service.file,
            &service.source,
            Scope::Singleton,
        )
    }

    pub(crate) fn root(path: &Path, file: &CppFile, source: &str) -> Result<Self, Failure> {
        CppService::check_needs(path, &file.class)?;
        CppLifetime::check(path, &file.class)?;
        Self::read(path, file, source, Scope::Request)
    }

    fn read(path: &Path, file: &CppFile, source: &str, unwritten: Scope) -> Result<Self, Failure> {
        let class = &file.class;
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
            lowered: CppProgram::located(path, &CppLowering::yon(file, source, Self::FRIENDS)),
        })
    }
}
