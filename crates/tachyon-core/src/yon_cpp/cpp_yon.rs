use super::{
    CppYonController, CppYonErrors, CppYonGraph, CppYonProgram, CppYonStores, CppYonTopics,
};
use crate::Failure;
use crate::behavior::GatewayDeclaration;
use crate::tac_cpp::{CppProgram, CppSource};
use crate::yon::Lowered;
use crate::yon::plan::Route;
use std::path::Path;

pub(crate) struct CppYon;

impl CppYon {
    pub(crate) fn lower(source: &Path, contents: &str, root: &Path) -> Result<Lowered, Failure> {
        let controller = CppYonController::read(source, contents)?;
        let (errors, models, graph) = Self::graph(root, &controller)?;
        Ok(Lowered {
            source: CppYonProgram::source(&controller, &graph, &models, &errors),
            topics: CppYonTopics::used(models.topics(), &graph),
            routes: controller
                .routes
                .into_iter()
                .filter(|route| route.pattern.is_some())
                .map(|route| Route {
                    error_responses: errors.responses(),
                    ..route
                })
                .collect(),
        })
    }

    pub(crate) fn gateways(
        root: &Path,
        caller: &Path,
        contents: &str,
        other: impl Fn(&Path, &str) -> Result<Vec<GatewayDeclaration>, Failure>,
    ) -> Result<Vec<GatewayDeclaration>, Failure> {
        Self::gateways_routed(CppSource::is_cpp(caller), root, caller, contents, other)
    }

    fn gateways_routed(
        cpp: bool,
        root: &Path,
        caller: &Path,
        contents: &str,
        other: impl Fn(&Path, &str) -> Result<Vec<GatewayDeclaration>, Failure>,
    ) -> Result<Vec<GatewayDeclaration>, Failure> {
        if cpp {
            return Self::declared(root, caller, contents);
        }
        other(caller, contents)
    }

    fn declared(
        root: &Path,
        caller: &Path,
        contents: &str,
    ) -> Result<Vec<GatewayDeclaration>, Failure> {
        let controller = CppYonController::read(caller, contents)?;
        Ok(Self::graph(root, &controller)?.2.gateways)
    }

    fn graph(
        root: &Path,
        controller: &CppYonController,
    ) -> Result<(CppYonErrors, CppProgram, CppYonGraph), Failure> {
        let errors = CppYonErrors::read(root, controller)?;
        let models = CppProgram::server_models(root)?;
        let stores = CppYonStores::read(root, &models)?;
        let graph = CppYonGraph::read(root, controller, &models, &errors, &stores)?;
        Ok((errors, models, graph))
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
