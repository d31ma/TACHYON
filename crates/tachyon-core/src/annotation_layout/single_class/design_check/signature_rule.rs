#![allow(clippy::needless_bitwise_bool)]

use super::super::cohesion::Method;
use super::class_file::ClassFile;
use super::dependency_graph::DependencyGraph;
use super::design_problem::DesignProblem;
use super::design_rule::DesignRule;
use super::method_filter::MethodFilter;
use super::name_forms::NameForms;
use super::parameter_list::ParameterList;
use super::signature::Signature;
use std::collections::BTreeSet;

pub(super) struct SignatureRule;

impl SignatureRule {
    const MOST_PARAMETERS: usize = 4;
    const MOST_DEPENDENCIES: usize = 5;
    const PROVIDED: [&'static str; 3] = ["DataSource", "Redis", "ApplicationEventPublisher"];

    pub(super) fn problems(classes: &[ClassFile], graph: &DependencyGraph) -> Vec<DesignProblem> {
        let parameters = classes.iter().flat_map(Self::parameter_problems);
        let dependencies = classes
            .iter()
            .enumerate()
            .filter_map(|(index, class)| Self::dependency_problem(index, class, graph));
        parameters.chain(dependencies).collect()
    }

    fn parameter_problems(class: &ClassFile) -> Vec<DesignProblem> {
        class
            .methods
            .iter()
            .filter(|method| !MethodFilter::is_constructor(class, method))
            .filter(|method| MethodFilter::names_are_chosen(class, method))
            .filter_map(|method| Self::parameter_problem(class, method))
            .collect()
    }

    fn parameter_problem(class: &ClassFile, method: &Method) -> Option<DesignProblem> {
        let signature = Signature::of(class, method);
        let count = ParameterList::read(class, signature.parameters)
            .iter()
            .filter(|parameter| parameter.counted)
            .count();
        let message = format!(
            "The method `{}` in {} takes {count} parameters; the limit is {}.",
            method.name,
            class.name,
            Self::MOST_PARAMETERS
        );
        let help = format!(
            "Group the values that belong together into a model (a class in {}/models) and take that model as one parameter. Parameters that Tachyon fills from the request (`FromPath`, `FromQuery`, `FromBody`, `FromHeader` and `FromCookie`) do not count.",
            class.realm
        );
        (count > Self::MOST_PARAMETERS)
            .then(|| class.problem(DesignRule::Signatures, signature.name_at, message, help))
    }

    fn dependency_problem(
        index: usize,
        class: &ClassFile,
        graph: &DependencyGraph,
    ) -> Option<DesignProblem> {
        let mut needed: Vec<String> = graph
            .needs
            .iter()
            .filter(|need| need.class == index)
            .map(|need| graph.names[need.to].clone())
            .collect();
        needed.extend(Self::provided(class));
        let message = format!(
            "{} needs {} classes ({}); the limit is {}.",
            class.name,
            needed.len(),
            needed.join(", "),
            Self::MOST_DEPENDENCIES
        );
        let help = String::from(
            "A class that needs this many others does too much. Split it into smaller classes that each need fewer, and let each caller use the part it needs.",
        );
        let at = class.first_word(class.name);
        (needed.len() > Self::MOST_DEPENDENCIES)
            .then(|| class.problem(DesignRule::Signatures, at, message, help))
    }

    fn provided(class: &ClassFile) -> BTreeSet<String> {
        Self::PROVIDED
            .iter()
            .filter(|name| Self::mentions(class, name))
            .map(|name| (*name).to_owned())
            .collect()
    }

    fn mentions(class: &ClassFile, name: &str) -> bool {
        let forms = NameForms::of(name);
        (0..class.words.len()).any(|at| {
            class.words.is_name(at) & forms.iter().any(|form| form == class.words.text(at))
        })
    }
}
