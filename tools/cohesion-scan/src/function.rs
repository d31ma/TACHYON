use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Construct {
    If,
    Match,
    Loop,
    LetElse,
    ShortCircuit,
    IteratorLogic,
}

impl Construct {
    const LOOPS: [Construct; 2] = [Construct::Loop, Construct::IteratorLogic];

    fn is_decision(self) -> bool {
        !Self::LOOPS.contains(&self)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Found {
    pub construct: Construct,
    pub line: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Violation {
    pub function: String,
    pub line: usize,
    pub public: bool,
    pub reasons: Vec<&'static str>,
    pub constructs: Vec<Found>,
}

#[derive(Debug)]
pub(crate) struct Function {
    name: String,
    line: usize,
    public: bool,
    constructs: Vec<Found>,
    calls: BTreeSet<String>,
}

impl Function {
    const TOO_MANY: &'static str = "too-many-constructs";
    const NESTED: &'static str = "branching-helper-calls-branching-helper";

    pub(crate) fn new(name: String, line: usize, public: bool) -> Self {
        Self {
            name,
            line,
            public,
            constructs: Vec::new(),
            calls: BTreeSet::new(),
        }
    }

    pub(crate) fn add_construct(&mut self, construct: Construct, line: usize) {
        self.constructs.push(Found { construct, line });
    }

    pub(crate) fn add_call(&mut self, name: String) {
        self.calls.insert(name);
    }

    pub(crate) fn construct_count(&self) -> usize {
        self.constructs.len()
    }

    pub(crate) fn without_self_calls(mut self) -> Self {
        self.calls.remove(&self.name);
        self
    }

    pub(crate) fn branching_helper_name(&self) -> Option<String> {
        let helper = [!self.public, self.decides()] == [true, true];
        helper.then(|| self.name.clone())
    }

    pub(crate) fn verdict(self, branching_helpers: &BTreeSet<String>) -> Option<Violation> {
        let checks = [
            (self.has_too_many_constructs(), Self::TOO_MANY),
            (self.calls_branching_helper(branching_helpers), Self::NESTED),
        ];
        let reasons: Vec<&'static str> = checks
            .into_iter()
            .filter(|(failed, _)| *failed)
            .map(|(_, reason)| reason)
            .collect();
        let failed = !reasons.is_empty();
        failed.then(|| self.into_violation(reasons))
    }

    fn into_violation(self, reasons: Vec<&'static str>) -> Violation {
        Violation {
            function: self.name,
            line: self.line,
            public: self.public,
            reasons,
            constructs: self.constructs,
        }
    }

    fn decides(&self) -> bool {
        self.constructs
            .iter()
            .any(|found| found.construct.is_decision())
    }

    fn has_too_many_constructs(&self) -> bool {
        self.constructs.len() > usize::from(!self.public)
    }

    fn calls_branching_helper(&self, branching_helpers: &BTreeSet<String>) -> bool {
        let calls_helper = !self.calls.is_disjoint(branching_helpers);
        [self.decides(), calls_helper] == [true, true]
    }
}
