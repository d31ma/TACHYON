use super::{CppYonContainer, CppYonController, CppYonDispatcher, CppYonGraph, CppYonRuntime};
use crate::tac_cpp::CppProgram;

pub(crate) struct CppYonProgram;

impl CppYonProgram {
    pub(crate) fn source(
        controller: &CppYonController,
        graph: &CppYonGraph,
        models: &CppProgram,
    ) -> String {
        [
            CppYonRuntime::source(),
            models.model_source(),
            models.setup_source(),
            CppYonContainer::declarations(graph),
            graph.located(),
            CppYonContainer::definitions(graph),
            CppYonDispatcher::source(controller),
        ]
        .concat()
    }
}
