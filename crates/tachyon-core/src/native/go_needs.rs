use crate::Failure;
use crate::go_source::GoTypeKind;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::ops::Range;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct GoNeeds {
    instances: Vec<(String, String)>,
    classes: BTreeSet<String>,
    providers: BTreeSet<String>,
    fields: BTreeSet<(String, String, String, String, usize)>,
    plans: BTreeMap<String, (Vec<String>, String)>,
    shared: BTreeSet<String>,
}

impl GoNeeds {
    const CODE: u16 = 1406;
    const MARKERS: [&'static str; 2] = ["@Needs", "@Needs()"];
    const FILE: &'static str = "tac_needs.go";

    pub(super) fn add(
        &mut self,
        (instance, class): (&str, &str),
        path: &str,
        source: &str,
    ) -> Result<(), Failure> {
        let roles = crate::behavior::ServiceRoles::read(Path::new(path), source)?;
        let provides = [roles.prototype, roles.required.is_empty()] == [false, true];
        let code = crate::companion::go::masked_code(source, path)?;
        self.classes.insert(class.to_owned());
        let fields = Self::marked(&code, Self::body(&code, class));
        self.fields.extend(Self::owned(class, path, &fields));
        self.providers.extend(provides.then(|| class.to_owned()));
        self.instances.push((instance.to_owned(), class.to_owned()));
        Ok(())
    }

    pub(super) fn resolve(&mut self) -> Result<(), Failure> {
        let needs = self.service_needs();
        let chosen = self.chosen(&needs)?;
        Self::refuse_circles(&needs)?;
        self.record(&needs, &chosen);
        Ok(())
    }

    pub(super) fn excluded(&self, class: &str) -> Vec<String> {
        self.plans
            .get(class)
            .map_or_else(Vec::new, |plan| plan.0.clone())
    }

    pub(super) fn statements(&self, class: &str) -> String {
        self.plans
            .get(class)
            .map_or_else(String::new, |plan| plan.1.clone())
    }

    pub(super) fn shared_file(&self, package: &str, module: &str) -> Option<(PathBuf, String)> {
        let calls = self.shared_calls();
        (!calls.is_empty()).then(|| {
            (
                Path::new(package).join(Self::FILE),
                format!(
                    "package {package}\n\nimport \"{module}/tac\"\n\nfunc init() {{\n{calls}}}\n"
                ),
            )
        })
    }

    fn shared_calls(&self) -> String {
        let mut calls = String::new();
        for instance in &self.shared {
            let _ = writeln!(calls, "\ttac.Shared({instance:?})");
        }
        calls
    }

    fn body(code: &str, class: &str) -> Range<usize> {
        crate::go_source::types(code)
            .into_iter()
            .find(|declared| {
                [declared.name == class, declared.kind == GoTypeKind::Struct] == [true, true]
            })
            .map_or(0..0, |declared| declared.body)
    }

    fn marked(code: &str, body: Range<usize>) -> Vec<(String, String, usize)> {
        let mut found = Vec::new();
        let mut marked = false;
        let mut at = body.start;
        for line in code[body].split_inclusive('\n') {
            marked = Self::step(&mut found, marked, line, at);
            at += line.len();
        }
        found
    }

    fn step(found: &mut Vec<(String, String, usize)>, marked: bool, line: &str, at: usize) -> bool {
        let text = line.trim();
        let opens = Self::MARKERS.contains(&text);
        let wanted = [marked, !opens, !text.is_empty()] == [true, true, true];
        let start = at + line.len() - line.trim_start().len();
        found.extend(wanted.then(|| Self::field(text, start)).flatten());
        [opens, [marked, text.is_empty()] == [true, true]].contains(&true)
    }

    fn field(text: &str, at: usize) -> Option<(String, String, usize)> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let [name, kind] = words.as_slice() else {
            return None;
        };
        Some(((*name).to_owned(), (*kind).to_owned(), at))
    }

    fn target(kind: &str) -> &str {
        kind.trim_start_matches('*')
    }

    fn service_needs(&self) -> Vec<(String, String, String, String, usize)> {
        self.fields
            .iter()
            .filter(|need| self.classes.contains(Self::target(&need.3)))
            .cloned()
            .collect()
    }

    fn owned(
        class: &str,
        path: &str,
        fields: &[(String, String, usize)],
    ) -> Vec<(String, String, String, String, usize)> {
        let mut found = Vec::new();
        for (field, kind, at) in fields {
            found.push((
                class.to_owned(),
                path.to_owned(),
                field.clone(),
                kind.clone(),
                *at,
            ));
        }
        found
    }

    fn chosen(
        &self,
        needs: &[(String, String, String, String, usize)],
    ) -> Result<Vec<String>, Failure> {
        needs.iter().map(|need| self.choose(need)).collect()
    }

    fn choose(&self, need: &(String, String, String, String, usize)) -> Result<String, Failure> {
        Self::require_pointer(need)?;
        let instance = self.instance(need)?;
        self.require_provider(need)?;
        Ok(instance)
    }

    fn require_pointer(need: &(String, String, String, String, usize)) -> Result<(), Failure> {
        let (owner, _, field, kind, _) = need;
        need.3.starts_with('*').then_some(()).ok_or_else(|| {
            Self::refused(
                need,
                &format!(
                    "Go service {owner} needs {kind} through a pointer: write `{field} *{kind}`. A value would be a copy, so {owner} and the page would not share one {kind}."
                ),
            )
        })
    }

    fn require_provider(
        &self,
        need: &(String, String, String, String, usize),
    ) -> Result<(), Failure> {
        let target = Self::target(&need.3);
        self.providers.contains(target).then_some(()).ok_or_else(|| {
            Self::refused(
                need,
                &format!(
                    "Go service {} cannot need {target}: {target} has one instance for each component, because it is Lifetime prototype or NeedsAll, so a field cannot name one of them. Make {target} a singleton without properties, or pass the value to a method of {}.",
                    need.0, need.0
                ),
            )
        })
    }

    fn instance(&self, need: &(String, String, String, String, usize)) -> Result<String, Failure> {
        let target = Self::target(&need.3);
        let candidates = self.candidates(target);
        let named = candidates
            .iter()
            .find(|instance| **instance == need.2)
            .cloned();
        let only = (candidates.len() == 1).then(|| candidates[0].clone());
        named.or(only).ok_or_else(|| {
            Self::refused(
                need,
                &format!(
                    "Go service {} cannot tell which {target} it needs: the page has {}. Name the field after one of them.",
                    need.0,
                    candidates.join(" and ")
                ),
            )
        })
    }

    fn candidates(&self, class: &str) -> Vec<String> {
        let mut found = Vec::new();
        for (instance, owned) in &self.instances {
            found.extend((owned == class).then(|| instance.clone()));
        }
        found
    }

    fn refuse_circles(needs: &[(String, String, String, String, usize)]) -> Result<(), Failure> {
        let edges = Self::edges(needs);
        needs
            .iter()
            .try_for_each(|need| Self::require_open(&edges, need))
    }

    fn edges(
        needs: &[(String, String, String, String, usize)],
    ) -> BTreeMap<String, BTreeSet<String>> {
        let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for need in needs {
            edges
                .entry(need.0.clone())
                .or_default()
                .insert(Self::target(&need.3).to_owned());
        }
        edges
    }

    fn require_open(
        edges: &BTreeMap<String, BTreeSet<String>>,
        need: &(String, String, String, String, usize),
    ) -> Result<(), Failure> {
        let target = Self::target(&need.3);
        let reached = Self::reached(edges, target);
        (!reached.contains(&need.0)).then_some(()).ok_or_else(|| {
            Self::refused(
                need,
                &format!(
                    "Go services {} and {target} need each other in a circle. Break the circle with an event: one service publishes it and the other listens with OnEvent.",
                    need.0
                ),
            )
        })
    }

    fn reached(edges: &BTreeMap<String, BTreeSet<String>>, from: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::from([from.to_owned()]);
        for _ in 0..edges.len() {
            seen = Self::widened(edges, &seen);
        }
        seen
    }

    fn widened(
        edges: &BTreeMap<String, BTreeSet<String>>,
        seen: &BTreeSet<String>,
    ) -> BTreeSet<String> {
        let mut next = seen.clone();
        for class in seen {
            next.extend(edges.get(class).into_iter().flatten().cloned());
        }
        next
    }

    fn record(&mut self, needs: &[(String, String, String, String, usize)], chosen: &[String]) {
        for (need, instance) in needs.iter().zip(chosen) {
            self.plan(need, instance);
        }
    }

    fn plan(&mut self, need: &(String, String, String, String, usize), instance: &str) {
        let plan = self.plans.entry(need.0.clone()).or_default();
        plan.0.push(need.2.clone());
        let _ = writeln!(
            plan.1,
            "\tinstance.{} = tac.Need({instance:?}).(*{})",
            need.2,
            Self::target(&need.3)
        );
        self.shared.insert(instance.to_owned());
    }

    fn refused(need: &(String, String, String, String, usize), message: &str) -> Failure {
        let (_, path, field, _, at) = need;
        Failure::one(crate::failure::diagnostic(
            Self::CODE,
            message,
            None,
            crate::failure::source_span(path, *at, at + field.len()),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::GoNeeds;

    struct Fixtures;

    impl Fixtures {
        const COUNTER: &str = "// @Service\ntype CounterService struct {\n\tCount int\n}\n";
        const TALLY: &str = "// @Service\ntype TallyService struct {\n\t// @Needs\n\tcounter *CounterService\n\t// @Needs\n\tstore *TallyStore\n\tTotal int\n}\n";

        fn page(services: &[(&str, &str, &str)]) -> GoNeeds {
            let mut needs = GoNeeds::default();
            for (instance, class, source) in services {
                let path = format!("client/services/{class}.go");
                needs
                    .add((instance, class), &path, source)
                    .expect("service");
            }
            needs
        }

        fn refusal(services: &[(&str, &str, &str)]) -> (String, String, u64) {
            let failure = Self::page(services).resolve().expect_err("refused");
            let diagnostic = &failure.diagnostics()[0];
            assert_eq!(diagnostic.code.to_string(), "TY1406");
            let span = &diagnostic.spans[0];
            (diagnostic.message.clone(), span.file.clone(), span.start)
        }
    }

    #[test]
    fn a_needed_singleton_is_shared_and_its_field_stays_off_the_page() {
        let mut needs = Fixtures::page(&[
            ("counter", "CounterService", Fixtures::COUNTER),
            ("tally", "TallyService", Fixtures::TALLY),
        ]);
        needs.resolve().expect("resolved");
        assert_eq!(
            needs.excluded("TallyService"),
            vec![String::from("counter")]
        );
        assert_eq!(
            needs.statements("TallyService"),
            "\tinstance.counter = tac.Need(\"counter\").(*CounterService)\n"
        );
        assert!(needs.excluded("CounterService").is_empty());
        assert!(needs.statements("CounterService").is_empty());
        let (path, text) = needs
            .shared_file("page", "tachyonservices")
            .expect("shared file");
        assert_eq!(
            path.to_string_lossy().replace('\\', "/"),
            "page/tac_needs.go"
        );
        assert_eq!(
            text,
            "package page\n\nimport \"tachyonservices/tac\"\n\nfunc init() {\n\ttac.Shared(\"counter\")\n}\n"
        );
    }

    #[test]
    fn a_page_without_service_needs_adds_no_file() {
        let mut needs = Fixtures::page(&[("counter", "CounterService", Fixtures::COUNTER)]);
        needs.resolve().expect("resolved");
        assert!(needs.shared_file("page", "tachyonservices").is_none());
    }

    #[test]
    fn a_needed_service_by_value_is_refused_at_the_field() {
        let tally = Fixtures::TALLY.replace("*CounterService", "CounterService");
        let (message, file, start) = Fixtures::refusal(&[
            ("counter", "CounterService", Fixtures::COUNTER),
            ("tally", "TallyService", &tally),
        ]);
        assert!(message.contains("through a pointer"), "{message}");
        assert!(message.contains("`counter *CounterService`"), "{message}");
        assert_eq!(file, "client/services/TallyService.go");
        let field = tally.find("counter CounterService").expect("field");
        assert_eq!(start, u64::try_from(field).expect("offset"));
    }

    #[test]
    fn a_needed_service_with_one_instance_per_component_is_refused() {
        for marker in [
            "// @Lifetime({\"value\":\"prototype\"})\n",
            "// @NeedsAll\n",
        ] {
            let counter = format!("{marker}{}", Fixtures::COUNTER);
            let (message, file, _) = Fixtures::refusal(&[
                ("counter", "CounterService", &counter),
                ("tally", "TallyService", Fixtures::TALLY),
            ]);
            assert!(
                message.contains("one instance for each component"),
                "{message}"
            );
            assert_eq!(file, "client/services/TallyService.go");
        }
    }

    #[test]
    fn services_that_need_each_other_in_a_circle_are_refused() {
        let counter = "// @Service\ntype CounterService struct {\n\t// @Needs\n\ttally *TallyService\n\tCount int\n}\n";
        let (message, _, _) = Fixtures::refusal(&[
            ("counter", "CounterService", counter),
            ("tally", "TallyService", Fixtures::TALLY),
        ]);
        assert!(message.contains("in a circle"), "{message}");
        let alone = "// @Service\ntype LoopService struct {\n\t// @Needs\n\tloop *LoopService\n\tCount int\n}\n";
        let (message, _, _) = Fixtures::refusal(&[("loop", "LoopService", alone)]);
        assert!(message.contains("in a circle"), "{message}");
    }

    #[test]
    fn two_instances_of_one_service_are_told_apart_by_the_field_name() {
        let services = [
            ("counter", "CounterService", Fixtures::COUNTER),
            ("counterTwin", "CounterService", Fixtures::COUNTER),
            ("tally", "TallyService", Fixtures::TALLY),
        ];
        let mut needs = Fixtures::page(&services);
        needs.resolve().expect("named instance");
        assert!(
            needs
                .statements("TallyService")
                .contains("tac.Need(\"counter\")")
        );
        let tally =
            Fixtures::TALLY.replace("\tcounter *CounterService", "\tclicks *CounterService");
        let (message, _, _) = Fixtures::refusal(&[
            ("counter", "CounterService", Fixtures::COUNTER),
            ("counterTwin", "CounterService", Fixtures::COUNTER),
            ("tally", "TallyService", &tally),
        ]);
        assert!(message.contains("counter and counterTwin"), "{message}");
    }
}
