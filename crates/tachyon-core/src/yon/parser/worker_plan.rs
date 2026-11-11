use super::{Input, ManagedType, add_resources, routes, validate_graph, words};
use crate::Failure;
use crate::yon::WorkerRoot;
use crate::yon::plan::Plan;

pub(super) struct WorkerPlan;

impl WorkerPlan {
    const MISSING_ROOT: &'static str = "A worker file holds one Handler class";

    pub(super) fn build(
        input: &Input<'_>,
        mut types: Vec<ManagedType>,
        controller: String,
    ) -> Result<Plan, Failure> {
        let root = types
            .iter()
            .position(|value| value.provider.name == controller);
        let managed = root
            .map(|index| &types[index])
            .ok_or_else(|| input.failure(0, Self::MISSING_ROOT))?;
        let route = routes::MappedRoutes::worker(input, managed)?;
        add_resources(input, &mut types)?;
        validate_graph(input, &types, root)?;
        Ok(Plan {
            providers: types.into_iter().map(|value| value.provider).collect(),
            controller,
            routes: vec![route],
            errors: Vec::new(),
        })
    }

    pub(super) fn declares_root(input: &Input<'_>, line: &str) -> bool {
        let declares = crate::annotation_layout::declares_type(input.extension, line);
        [declares, Self::names_root(input, line)] == [true, true]
    }

    fn names_root(input: &Input<'_>, line: &str) -> bool {
        words(line).any(|word| WorkerRoot::is_root(input.path, word))
    }
}
