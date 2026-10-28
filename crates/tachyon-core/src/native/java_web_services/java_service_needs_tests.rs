#![allow(clippy::expect_used)]

use super::java_constructor_provider::JavaConstructorProvider;
use super::java_service::JavaService;
use super::java_service_needs::JavaServiceNeeds;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

struct NeedsFixture;

impl NeedsFixture {
    const COUNTER: &'static str = "@Service\nclass CounterService {\n    int count = 0;\n}\n";
    const TALLY: &'static str = "@Service\nclass TallyService {\n    @Needs\n    private final CounterService counter;\n}\n";

    fn page(services: &[(&str, &str, &str)]) -> JavaServiceNeeds {
        let (entries, texts) = Self::entries(services);
        JavaServiceNeeds::read(&entries, &texts).expect("page")
    }

    fn entries(services: &[(&str, &str, &str)]) -> (Vec<(String, String, PathBuf)>, Vec<String>) {
        services
            .iter()
            .map(|(instance, class, source)| {
                (
                    (
                        String::from(*instance),
                        String::from(*class),
                        PathBuf::from(format!("client/services/{class}.java")),
                    ),
                    String::from(*source),
                )
            })
            .unzip()
    }

    fn provider(
        needs: &JavaServiceNeeds,
        (class, source): (&str, &str),
        (kind, field): (&str, &str),
    ) -> Result<Option<JavaConstructorProvider>, crate::Failure> {
        let path = PathBuf::from(format!("client/services/{class}.java"));
        let lowered = crate::behavior::ServiceRoles::read(&path, source)
            .expect("roles")
            .source;
        let owner = needs
            .owner("owner", &path, (source, &lowered, class))
            .expect("owner");
        owner.provider(kind, field)
    }

    fn refusal(
        needs: &JavaServiceNeeds,
        (class, source): (&str, &str),
        (kind, field): (&str, &str),
    ) -> (String, u64) {
        let failure = Self::provider(needs, (class, source), (kind, field)).expect_err("refused");
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.to_string(), "TY1406");
        (diagnostic.message.clone(), diagnostic.spans[0].start)
    }

    fn service(instance: &str, needs: &[(&str, &str)]) -> JavaService {
        JavaService {
            instance: String::from(instance),
            class: format!("{instance}Service"),
            members: Vec::new(),
            required: Vec::new(),
            providers: needs
                .iter()
                .map(|(field, target)| JavaConstructorProvider::Service {
                    name: String::from(*field),
                    instance: String::from(*target),
                    class: String::from("TacService0.Target"),
                })
                .collect(),
            path: Path::new("client/services").join(format!("{instance}Service.java")),
            source: String::new(),
        }
    }
}

#[test]
fn a_needed_singleton_service_is_found_by_its_type_and_built_in_its_holder() {
    let needs = NeedsFixture::page(&[
        ("counter", "CounterService", NeedsFixture::COUNTER),
        ("tally", "TallyService", NeedsFixture::TALLY),
    ]);
    let provider = NeedsFixture::provider(
        &needs,
        ("TallyService", NeedsFixture::TALLY),
        ("CounterService", "counter"),
    )
    .expect("provider");
    assert_eq!(
        provider,
        Some(JavaConstructorProvider::Service {
            name: String::from("counter"),
            instance: String::from("counter"),
            class: String::from("TacService0.CounterService"),
        })
    );
    let expression = provider.expect("service").expression();
    assert_eq!(
        expression,
        "(TacService0.CounterService) TacNeeds.need(\"counter\")"
    );
    let unknown = NeedsFixture::provider(
        &needs,
        ("TallyService", NeedsFixture::TALLY),
        ("String", "label"),
    )
    .expect("not a service");
    assert!(unknown.is_none());
}

#[test]
fn named_picks_the_provider_class_for_a_field_of_its_base_class() {
    let loud = "@Service\nclass LoudCounterService extends CounterService {\n}\n";
    let welcome = "@Service\nclass WelcomeService {\n    @Needs\n    @Named(\"LoudCounterService\")\n    private final CounterService counter;\n}\n";
    let needs = NeedsFixture::page(&[
        ("counter", "CounterService", NeedsFixture::COUNTER),
        ("loudCounter", "LoudCounterService", loud),
        ("welcome", "WelcomeService", welcome),
    ]);
    let provider = NeedsFixture::provider(
        &needs,
        ("WelcomeService", welcome),
        ("CounterService", "counter"),
    )
    .expect("named provider")
    .expect("service");
    assert_eq!(
        provider.needed(),
        Some(("loudCounter", "TacService1.LoudCounterService"))
    );
    let missing = welcome.replace("LoudCounterService", "QuietCounterService");
    let (message, start) = NeedsFixture::refusal(
        &needs,
        ("WelcomeService", &missing),
        ("CounterService", "counter"),
    );
    assert!(
        message.contains("no Java service QuietCounterService"),
        "{message}"
    );
    let field = missing.find("counter;").expect("field");
    assert_eq!(start, u64::try_from(field).expect("offset"));
}

#[test]
fn a_needed_service_with_one_instance_per_component_is_refused() {
    for marker in ["@Lifetime({\"value\":\"prototype\"})\n", "@NeedsAll\n"] {
        let counter = format!("{marker}{}", NeedsFixture::COUNTER);
        let needs = NeedsFixture::page(&[
            ("counter", "CounterService", &counter),
            ("tally", "TallyService", NeedsFixture::TALLY),
        ]);
        let (message, _) = NeedsFixture::refusal(
            &needs,
            ("TallyService", NeedsFixture::TALLY),
            ("CounterService", "counter"),
        );
        assert!(
            message.contains("one instance for each component"),
            "{message}"
        );
    }
}

#[test]
fn two_instances_of_one_service_are_told_apart_by_the_field_name() {
    let needs = NeedsFixture::page(&[
        ("counter", "CounterService", NeedsFixture::COUNTER),
        ("counterTwin", "CounterService", NeedsFixture::COUNTER),
        ("tally", "TallyService", NeedsFixture::TALLY),
    ]);
    let named = NeedsFixture::provider(
        &needs,
        ("TallyService", NeedsFixture::TALLY),
        ("CounterService", "counterTwin"),
    )
    .expect("field name")
    .expect("service");
    assert_eq!(
        named.needed(),
        Some(("counterTwin", "TacService1.CounterService"))
    );
    let (message, _) = NeedsFixture::refusal(
        &needs,
        ("TallyService", NeedsFixture::TALLY),
        ("CounterService", "clicks"),
    );
    assert!(message.contains("counter and counterTwin"), "{message}");
}

#[test]
fn services_that_need_each_other_in_a_circle_are_refused() {
    let services = [
        NeedsFixture::service("counter", &[("tally", "tally")]),
        NeedsFixture::service("tally", &[("counter", "counter")]),
    ];
    let texts = [
        String::from("@Service\nclass counterService {}\n"),
        String::from("@Service\nclass tallyService {}\n"),
    ];
    let failure =
        JavaServiceNeeds::shared(&services, &texts, &BTreeMap::new()).expect_err("circle");
    assert!(failure.to_string().contains("in a circle"), "{failure}");
    let alone = [NeedsFixture::service("loop", &[("again", "loop")])];
    let failure =
        JavaServiceNeeds::shared(&alone, &texts[..1], &BTreeMap::new()).expect_err("self");
    assert!(failure.to_string().contains("in a circle"), "{failure}");
    let line = [
        NeedsFixture::service("counter", &[]),
        NeedsFixture::service("tally", &[("counter", "counter")]),
    ];
    let shared = JavaServiceNeeds::shared(&line, &texts, &BTreeMap::new()).expect("no circle");
    assert_eq!(shared.into_iter().collect::<Vec<_>>(), ["counter"]);
}

#[test]
fn other_page_service_classes_are_named_through_their_holder() {
    let needs = NeedsFixture::page(&[
        ("counter", "CounterService", NeedsFixture::COUNTER),
        ("tally", "TallyService", NeedsFixture::TALLY),
    ]);
    let source = "class TallyService extends CounterService {\n    final CounterService counter;\n    String label = \"CounterService\";\n    MyCounterService other;\n    x.CounterService qualified;\n    TallyService(CounterService counter) {}\n}\n";
    let qualified = needs.qualified(source, "TallyService", &[]);
    assert_eq!(
        qualified,
        "class TallyService extends TacService0.CounterService {\n    final TacService0.CounterService counter;\n    String label = \"CounterService\";\n    MyCounterService other;\n    x.CounterService qualified;\n    TallyService(TacService0.CounterService counter) {}\n}\n"
    );
    assert_eq!(qualified.lines().count(), source.lines().count());
}
