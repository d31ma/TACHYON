#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;
use super::dependency_graph::DependencyGraph;
use super::design_problem::DesignProblem;
use super::design_rule::DesignRule;
use super::need::Need;
use super::role_layer::RoleLayer;

pub(super) struct LayerReport;

impl LayerReport {
    const MESSAGES: [&'static str; 3] = [
        "{class} needs {target}, but {target} is a controller, and nothing may need a controller.",
        "{class} is {label}, and it needs {target}, which is {target_label}. Dependencies point down: controllers, then services, then delegates, then stores, repositories and clients.",
        "{class} is a utility, and it names {target}, which is {target_label}. A utility depends on no role class.",
    ];
    const HELPS: [&'static str; 3] = [
        "A controller is an entry point: only a request calls it. Move the code that {class} uses into a service, and let the controller and {class} both use that service.",
        "A lower layer is used by the layers above it, not the other way round. Move the code of {class} that needs {target} up into {target_label} that uses {class}.",
        "Every class of the realm can use a utility, so a utility holds only plain static code (ADR 0064). Pass the values that {class} needs as parameters, or move this code into {target_label} or a service.",
    ];
    const UTILITY_MESSAGE: usize = 2;

    pub(super) fn problems(graph: &DependencyGraph, classes: &[ClassFile]) -> Vec<DesignProblem> {
        let layers = graph
            .needs
            .iter()
            .filter_map(|need| Self::problem(graph, classes, need));
        let utilities = graph
            .utility_needs
            .iter()
            .map(|need| Self::reported(graph, classes, need, Self::UTILITY_MESSAGE));
        layers.chain(utilities).collect()
    }

    fn problem(
        graph: &DependencyGraph,
        classes: &[ClassFile],
        need: &Need,
    ) -> Option<DesignProblem> {
        let class = &classes[need.class];
        let target_folder = &graph.folders[need.to];
        let broken = Self::broken(RoleLayer::of(class.folder), RoleLayer::of(target_folder))?;
        Some(Self::reported(graph, classes, need, broken))
    }

    fn reported(
        graph: &DependencyGraph,
        classes: &[ClassFile],
        need: &Need,
        broken: usize,
    ) -> DesignProblem {
        let class = &classes[need.class];
        let target_folder = &graph.folders[need.to];
        let fill = |text: &str| {
            text.replace("{class}", class.name)
                .replace("{target}", &graph.names[need.to])
                .replace("{label}", RoleLayer::label(class.folder))
                .replace("{target_label}", RoleLayer::label(target_folder))
        };
        class.problem(
            DesignRule::Dependencies,
            need.at,
            fill(Self::MESSAGES[broken]),
            fill(Self::HELPS[broken]),
        )
    }

    fn broken(from: RoleLayer, to: RoleLayer) -> Option<usize> {
        let controller = to == RoleLayer::EntryPoint;
        let upward = from
            .rank()
            .zip(to.rank())
            .is_some_and(|(from_rank, to_rank)| to_rank < from_rank);
        [controller.then_some(0), upward.then_some(1)]
            .into_iter()
            .flatten()
            .next()
    }
}
