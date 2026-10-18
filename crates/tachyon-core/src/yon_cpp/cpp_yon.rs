use super::{CppYonController, CppYonErrors, CppYonGraph, CppYonProgram};
use crate::Failure;
use crate::tac_cpp::{CppProgram, CppSource};
use crate::yon::Lowered;
use crate::yon::plan::Route;
use std::path::Path;

pub(crate) struct CppYon;

impl CppYon {
    pub(crate) fn lower(source: &Path, contents: &str, root: &Path) -> Result<Lowered, Failure> {
        let controller = CppYonController::read(source, contents)?;
        let errors = CppYonErrors::read(root, &controller)?;
        let models = CppProgram::server_models(root)?;
        let graph = CppYonGraph::read(root, &controller, &models, &errors)?;
        Ok(Lowered {
            source: CppYonProgram::source(&controller, &graph, &models, &errors),
            routes: controller
                .routes
                .into_iter()
                .filter(|route| route.pattern.is_some())
                .map(|route| Route {
                    error_responses: errors.responses(),
                    ..route
                })
                .collect(),
            topics: crate::events::TopicUse::default(),
        })
    }

    pub(crate) fn either(
        source: &Path,
        contents: &str,
        other: impl Fn(&Path, &str) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        Self::routed(CppSource::is_cpp(source), source, contents, other)
    }

    fn routed(
        cpp: bool,
        source: &Path,
        contents: &str,
        other: impl Fn(&Path, &str) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        if cpp {
            return CppYonController::read(source, contents).map(drop);
        }
        other(source, contents)
    }
}
