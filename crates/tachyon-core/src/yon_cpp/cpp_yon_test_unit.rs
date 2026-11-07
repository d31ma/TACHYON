use super::{
    CppYonContainer, CppYonErrors, CppYonGraph, CppYonRuntime, CppYonStores, CppYonTopics,
};
use crate::Failure;
use crate::behavior::GatewayDeclaration;
use crate::tac_cpp::{CppProgram, CppTestUnit};
use crate::yon::Lowered;
use std::fmt::Write as _;
use std::path::Path;

pub(crate) struct CppYonTestUnit;

impl CppYonTestUnit {
    const FOLDER: &'static str = "server/tests";
    const START: &'static str = "#line 1 \"tachyon-yon-tests.cpp\"\n";

    pub(crate) fn is_test(path: &Path) -> bool {
        path.starts_with(Self::FOLDER)
    }

    pub(crate) fn lower(source: &Path, contents: &str, root: &Path) -> Result<Lowered, Failure> {
        let unit = CppTestUnit::read(root, source, contents)?;
        let (models, errors, graph) = Self::graph(root)?;
        unit.check_fake_types(|name| graph.has_class(name))?;
        Ok(Lowered {
            source: [
                CppYonRuntime::tested(),
                models.model_source(),
                errors.type_source(),
                models.setup_source(),
                CppYonTopics::source(&models.topics()),
                CppYonContainer::declarations(&graph),
                graph.located(),
                CppYonContainer::faked_definitions(&graph),
                unit.source().to_owned(),
                Self::runner(&unit),
            ]
            .concat(),
            routes: Vec::new(),
            topics: crate::events::TopicUse::default(),
        })
    }

    pub(crate) fn gateways(
        root: &Path,
        _caller: &Path,
        _contents: &str,
    ) -> Result<Vec<GatewayDeclaration>, Failure> {
        Ok(Self::graph(root)?.2.gateways)
    }

    fn graph(root: &Path) -> Result<(CppProgram, CppYonErrors, CppYonGraph), Failure> {
        let models = CppProgram::server_models(root)?;
        let errors = CppYonErrors::every(root)?;
        let stores = CppYonStores::read(root, &models)?;
        let graph = CppYonGraph::tested(root, &models, &errors, &stores)?;
        Ok((models, errors, graph))
    }

    fn runner(unit: &CppTestUnit) -> String {
        let opened = unit.opened();
        let mut cases = String::new();
        for case in &unit.cases {
            let _ = writeln!(
                cases,
                "        runner.run(\"{}\", [] {{ {opened}tachyon::yon::YonRemote::answering() = &answers.context(); tachyon::yon::YonInvocation invocation; {} unit; tachyon::TacTestCase::attach(unit, answers.context()); invocation.start(); tachyon::TacTestCase::finish([&unit] {{ return unit.{}; }}); invocation.drain(); tachyon::yon::YonRemote::answering() = nullptr; }});",
                case.name, unit.name, case.call
            );
        }
        format!(
            "{start}class TachyonYonRoutes {{\npublic:\n    static tachyon::yon::YonAnswer answer(const tachyon::yon::YonFrame&) {{\n        tachyon::TacTestRunner runner(\"{name}\");\n{cases}        return tachyon::yon::YonAnswer::json(200, runner.text());\n    }}\n}};\n\nint main() {{\n    return tachyon::yon::YonMain::run<TachyonYonRoutes>();\n}}\n",
            start = Self::START,
            name = unit.name,
        )
    }
}
