use super::breadth_search::BreadthSearch;
use super::class_file::ClassFile;
use super::dependency_graph::DependencyGraph;
use super::design_problem::DesignProblem;
use super::design_rule::DesignRule;
use super::layer_report::LayerReport;

pub(super) struct DependencyRule;

impl DependencyRule {
    const HELP: &'static str = "Break the loop: move the code that these classes share into a new service that each of them uses, or let one class publish an event instead of calling back.";

    pub(super) fn problems(graph: &DependencyGraph, classes: &[ClassFile]) -> Vec<DesignProblem> {
        let mut problems = Self::cycles(graph, classes);
        problems.extend(LayerReport::problems(graph, classes));
        problems
    }

    fn cycles(graph: &DependencyGraph, classes: &[ClassFile]) -> Vec<DesignProblem> {
        (0..graph.names.len())
            .filter_map(|start| BreadthSearch::cycle(&graph.edges, start))
            .filter_map(|cycle| Self::cycle_problem(graph, classes, &cycle))
            .collect()
    }

    fn cycle_problem(
        graph: &DependencyGraph,
        classes: &[ClassFile],
        cycle: &[usize],
    ) -> Option<DesignProblem> {
        let need = graph.need_between(cycle[0], cycle[1])?;
        let names: Vec<&str> = cycle
            .iter()
            .map(|&node| graph.names[node].as_str())
            .collect();
        let message = format!(
            "Circular dependency: {}. Each class in this loop needs the next one, so none of them can be built first.",
            names.join(" -> ")
        );
        let class = &classes[need.class];
        Some(class.problem(
            DesignRule::Dependencies,
            need.at,
            message,
            Self::HELP.to_owned(),
        ))
    }
}
