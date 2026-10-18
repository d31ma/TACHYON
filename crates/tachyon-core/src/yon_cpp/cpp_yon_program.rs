use super::{
    CppYonContainer, CppYonController, CppYonDispatcher, CppYonErrors, CppYonGraph, CppYonRuntime,
};
use crate::tac_cpp::CppProgram;

pub(crate) struct CppYonProgram;

impl CppYonProgram {
    pub(crate) fn source(
        controller: &CppYonController,
        graph: &CppYonGraph,
        models: &CppProgram,
        errors: &CppYonErrors,
    ) -> String {
        [
            CppYonRuntime::source(),
            models.model_source(),
            errors.type_source(),
            models.setup_source(),
            CppYonContainer::declarations(graph),
            graph.located(),
            CppYonContainer::definitions(graph),
            CppYonDispatcher::source(controller, errors),
        ]
        .concat()
    }
}
