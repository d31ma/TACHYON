use super::{
    CppYonController, CppYonErrors, CppYonGatewayUnit, CppYonGraph, CppYonProgram, CppYonStores,
    CppYonTestUnit, CppYonTopics,
};
use crate::Failure;
use crate::behavior::{GatewayDeclaration, TargetSignature};
use crate::events::TopicUse;
use crate::tac_cpp::{CppProgram, CppSource, CppTestUnit};
use crate::yon::Lowered;
use crate::yon::plan::Route;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) struct CppYon;

impl CppYon {
    pub(crate) fn lower(source: &Path, contents: &str, root: &Path) -> Result<Lowered, Failure> {
        let lower =
            [Self::controller, CppYonTestUnit::lower][usize::from(CppYonTestUnit::is_test(source))];
        lower(source, contents, root)
    }

    pub(crate) fn middleware(
        middleware: bool,
        source: &Path,
        contents: &str,
        root: &Path,
    ) -> Result<(Option<String>, Vec<Route>, TopicUse), Failure> {
        let cpp = [middleware, CppSource::is_cpp(source)] == [true, true];
        let lowered = cpp
            .then(|| Self::lower(source, contents, root))
            .transpose()?;
        Ok(lowered.map_or_else(
            || (None, Vec::new(), TopicUse::default()),
            |lowered| (Some(lowered.source), Vec::new(), lowered.topics),
        ))
    }

    fn controller(source: &Path, contents: &str, root: &Path) -> Result<Lowered, Failure> {
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
        let declared = [Self::controller_gateways, CppYonTestUnit::gateways]
            [usize::from(CppYonTestUnit::is_test(caller))];
        declared(root, caller, contents)
    }

    fn controller_gateways(
        root: &Path,
        caller: &Path,
        contents: &str,
    ) -> Result<Vec<GatewayDeclaration>, Failure> {
        let controller = CppYonController::read(caller, contents)?;
        Ok(Self::graph(root, &controller)?.2.gateways)
    }

    pub(crate) fn gateway_unit(
        root: &Path,
        unit: &Path,
        target: &str,
        methods: &BTreeMap<String, usize>,
        other: impl Fn() -> Result<(String, String), Failure>,
    ) -> Result<(String, String), Failure> {
        let lowered = CppSource::is_cpp(unit)
            .then(|| CppYonGatewayUnit::lower(root, unit, target, methods))
            .transpose()?;
        lowered.map_or_else(other, |lowered| Ok((lowered.clone(), lowered)))
    }

    pub(crate) fn gateway_target_signatures(
        path: &Path,
        source: &str,
        class: &str,
        method: &str,
    ) -> Result<Vec<TargetSignature>, Failure> {
        CppYonGatewayUnit::signatures(path, source, class, method)
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

    pub(crate) fn streaming(source: &Path, contents: &str) -> Option<BTreeSet<String>> {
        let read = CppSource::is_cpp(source).then(|| CppYonController::read(source, contents).ok());
        read.flatten().map(|controller| {
            controller
                .routes
                .into_iter()
                .filter(|route| route.streaming)
                .map(|route| route.verb)
                .collect()
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
            let check = [Self::read_controller, CppTestUnit::check]
                [usize::from(CppYonTestUnit::is_test(source))];
            return check(source, contents);
        }
        other(source, contents)
    }

    fn read_controller(source: &Path, contents: &str) -> Result<(), Failure> {
        CppYonController::read(source, contents).map(drop)
    }
}
