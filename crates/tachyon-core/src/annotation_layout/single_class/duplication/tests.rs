#![allow(clippy::expect_used)]

use super::DuplicationCheck;
use super::RuleSwitch;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

fn javascript_service(name: &str, subject: &str, value: &str) -> String {
    let lines = (0..10)
        .map(|line| format!("    const {subject}{line} = {subject}.read(\"{value}\", {line});\n"))
        .collect::<Vec<_>>()
        .concat();
    format!(
        "@Service\nexport default class {name} {{\n  total({subject}) {{\n{lines}    return {subject}\n  }}\n}}\n"
    )
}

fn found(files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let pairs = files
        .iter()
        .map(|(path, text)| (Path::new(*path), text.as_bytes()));
    DuplicationCheck::diagnostics(pairs, RuleSwitch::every_realm())
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(path, contents).expect("file");
}

#[test]
fn a_block_repeated_with_other_names_names_both_places_and_a_utility() {
    let order = javascript_service("OrderService", "order", "a");
    let invoice = javascript_service("OrderHistoryService", "invoice", "b");
    let problems = found(&[
        ("server/services/OrderService.js", &order),
        ("server/services/OrderHistoryService.js", &invoice),
    ]);
    assert_eq!(problems.len(), 1, "{problems:?}");
    let problem = &problems[0];
    assert_eq!(problem.code.number(), 1315);
    assert_eq!(
        problem.message,
        "Duplicated code: lines 2-16 repeat server/services/OrderService.js:2-16 (134 tokens; only names and values may differ)."
    );
    assert_eq!(
        problem.spans[0].file,
        "server/services/OrderHistoryService.js"
    );
    let help = problem.help.as_deref().unwrap_or("");
    assert!(
        help.contains("create server/utils/OrderUtils.js with a static method"),
        "{help}"
    );
    assert!(
        help.contains("`duplication.check` in server/application.json"),
        "{help}"
    );
}

#[test]
fn realms_and_languages_do_not_share_duplicates() {
    let service = javascript_service("OrderService", "order", "a");
    let problems = found(&[
        ("client/services/OrderService.js", &service),
        ("server/services/OrderService.js", &service),
        ("server/services/OrderService.ts", &service),
    ]);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn tests_models_and_generated_files_are_not_checked() {
    let service = javascript_service("OrderService", "order", "a");
    let problems = found(&[
        ("client/tests/OrderServiceTest.js", &service),
        ("client/models/Order.js", &service),
        ("client/services/Order.generated.js", &service),
        ("client/shared/scripts/chart.min.js", &service),
        ("client/shared/assets/chart.js", &service),
        ("client/services/OrderService.js", &service),
    ]);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn imports_annotations_and_comments_do_not_count() {
    let header = (0..12)
        .map(|line| format!("from shop.part{line} import first, second, third, fourth\n"))
        .collect::<Vec<_>>()
        .concat();
    let annotations = (0..12)
        .map(|line| format!("@Route(\"/item/{line}\", method=\"GET\", cache=True)\n"))
        .collect::<Vec<_>>()
        .concat();
    let comments = (0..12)
        .map(|line| format!("# the same comment on every line {line} of the file\n"))
        .collect::<Vec<_>>()
        .concat();
    let first = format!("{header}{annotations}{comments}class OrderService:\n    pass\n");
    let second = format!("{header}{annotations}{comments}class ItemService:\n    pass\n");
    let problems = found(&[
        ("server/services/OrderService.py", &first),
        ("server/services/ItemService.py", &second),
    ]);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn rust_unrelated_immutable_labels_and_short_accessors_are_not_duplicate_behavior() {
    let title = include_str!("../../../../tests/fixtures/rust-constant-duplication/TitleUtils.rs");
    let metrics =
        include_str!("../../../../tests/fixtures/rust-constant-duplication/MetricLabelsUtils.rs");
    assert!(
        found(&[
            ("client/utils/TitleUtils.rs", title),
            ("client/utils/MetricLabelsUtils.rs", metrics)
        ])
        .is_empty()
    );
    for (name, source) in [("TitleUtils", title), ("MetricLabelsUtils", metrics)] {
        crate::annotation_layout::validate(Path::new(&format!("client/utils/{name}.rs")), source)
            .expect("real stateless utility");
    }
}

#[test]
fn javascript_monitor_constant_tables_are_not_duplicate_behavior() {
    let graph = include_str!(
        "../../../../tests/fixtures/javascript-constant-duplication/GraphGroupRenderer.js"
    );
    let report = include_str!(
        "../../../../tests/fixtures/javascript-constant-duplication/NativePreviewReport.js"
    );
    let problems = found(&[
        ("client/shared/scripts/GraphGroupRenderer.js", graph),
        ("client/shared/native/NativePreviewReport.js", report),
    ]);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn javascript_constant_tables_do_not_hide_repeated_methods() {
    let first = javascript_service("FirstService", "order", "a");
    let second = javascript_service("SecondService", "invoice", "b");
    let problems = found(&[
        ("client/services/FirstService.js", &first),
        ("client/services/SecondService.js", &second),
    ]);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].code.number(), 1315);
    assert_eq!(tachyon_duplication::DuplicateFinder::MIN_TOKENS, 100);
    assert_eq!(tachyon_duplication::DuplicateFinder::MIN_LINES, 10);
}

fn script_initializers(initializer: &str, static_member: bool) -> String {
    let prefix = if static_member { "static " } else { "" };
    let fields = (0..24)
        .map(|index| format!("    {prefix}#VALUE_{index} = {initializer};\n"))
        .collect::<Vec<_>>()
        .concat();
    format!("class First {{\n{fields}}}\n")
}

#[test]
fn typescript_literal_constants_follow_the_same_declaration_contract() {
    let first = script_initializers("'value'", true)
        .replace("static #", "private static readonly ")
        .replace(" = ", ": string = ");
    let second = first.replace("First", "Second");
    assert!(
        found(&[
            ("client/shared/First.ts", &first),
            ("client/shared/Second.ts", &second),
        ])
        .is_empty()
    );
    let computed = first.replace("'value'", "make('value')");
    let other = computed.replace("First", "Second");
    assert!(
        !found(&[
            ("client/shared/First.ts", &computed),
            ("client/shared/Second.ts", &other),
        ])
        .is_empty()
    );
}

#[test]
fn javascript_computed_and_instance_fields_still_count_as_code() {
    for (initializer, static_member) in [
        ("make('value')", true),
        ("{ value: 'value', name: 'name' }", true),
        ("`value ${make()}`", true),
        ("'value'", false),
    ] {
        let first = script_initializers(initializer, static_member);
        let second = first.replace("First", "Second");
        let problems = found(&[
            ("client/shared/First.js", &first),
            ("client/shared/Second.js", &second),
        ]);
        assert!(!problems.is_empty(), "{initializer}: {problems:?}");
        assert_eq!(problems[0].code.number(), 1315);
    }
}

fn rust_calculation() -> String {
    (0..12)
        .map(|step| {
            let prior = if step == 0 {
                String::from("value")
            } else {
                format!("step{}", step - 1)
            };
            format!("        let step{step} = {prior}.wrapping_add({step}).wrapping_mul(7);\n")
        })
        .collect::<Vec<_>>()
        .concat()
}

#[test]
fn rust_substantive_repeated_methods_still_exceed_the_unchanged_threshold() {
    let body = rust_calculation();
    let first = format!(
        "pub struct FirstUtils;\nimpl FirstUtils {{\n    pub fn score(value: u64) -> u64 {{\n{body}        step11\n    }}\n}}\n"
    );
    let second = first.replace("FirstUtils", "SecondUtils");
    let diagnostics = found(&[
        ("client/utils/FirstUtils.rs", &first),
        ("client/utils/SecondUtils.rs", &second),
    ]);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code.number(), 1315);
    assert!(diagnostics[0].spans[0].start > 0);
    assert_eq!(tachyon_duplication::DuplicateFinder::MIN_TOKENS, 100);
    assert_eq!(tachyon_duplication::DuplicateFinder::MIN_LINES, 10);
}

#[test]
fn rust_computed_constant_bodies_still_count_as_behavior() {
    let body = rust_calculation();
    let first = format!(
        "pub struct FirstUtils;\nimpl FirstUtils {{\n    const COMPUTED: u64 = {{\n        let value = 1_u64;\n{body}        step11\n    }};\n    pub fn score() -> u64 {{ Self::COMPUTED }}\n}}\n"
    );
    let second = first.replace("FirstUtils", "SecondUtils");
    let diagnostics = found(&[
        ("client/utils/FirstUtils.rs", &first),
        ("client/utils/SecondUtils.rs", &second),
    ]);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code.number(), 1315);
}

#[test]
fn every_language_reads_its_tokens() {
    let block = (0..10)
        .map(|line| format!("    total{line} = order.read(\"v\", {line}, total{line}, 7)\n"))
        .collect::<Vec<_>>()
        .concat();
    for extension in [
        "py", "rb", "go", "rs", "kt", "swift", "dart", "cs", "java", "php",
    ] {
        let opening = ["", "<?php\n"][usize::from(extension == "php")];
        let first = format!("{opening}{block}start\n");
        let second = format!("{opening}finish\n{block}");
        let problems = found(&[
            (&format!("server/services/AService.{extension}"), &first),
            (&format!("server/services/BService.{extension}"), &second),
        ]);
        assert_eq!(problems.len(), 1, "{extension}: {problems:?}");
    }
}

#[test]
fn migration_lists_duplicates_even_when_the_check_is_off() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/manifest.json",
        r#"{"tac":{"duplication":{"check":false}}}"#,
    );
    write(
        root.path(),
        "client/services/OrderService.ts",
        &javascript_service("OrderService", "order", "a"),
    );
    write(
        root.path(),
        "client/services/ItemService.ts",
        &javascript_service("ItemService", "item", "b"),
    );
    let findings = DuplicationCheck::migration_findings(root.path());
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].feature, "duplication.block");
    assert_eq!(findings[0].source, "client/services/OrderService.ts");
    let action = findings[0].action.as_deref().unwrap_or("");
    assert!(action.contains("client/utils/SharedUtils.ts"), "{action}");
}

#[test]
fn discovery_refuses_duplicated_code_until_the_realm_opts_out() {
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "client/pages/tac.html", "<p>Home</p>");
    write(
        root.path(),
        "client/services/OrderService.js",
        &javascript_service("OrderService", "order", "a"),
    );
    write(
        root.path(),
        "client/services/ItemService.js",
        &javascript_service("ItemService", "item", "b"),
    );
    let refused = crate::ProjectDiscovery::discover(root.path()).expect_err("TY1315");
    assert!(
        refused
            .diagnostics()
            .iter()
            .any(|found| found.code.number() == 1315),
        "{:?}",
        refused.diagnostics()
    );
    write(
        root.path(),
        "client/manifest.json",
        r#"{"tac":{"duplication":{"check":false},"constants":{"check":false},"visibility":{"check":false}}}"#,
    );
    let allowed = crate::ProjectDiscovery::discover(root.path()).map(|_| ());
    assert!(allowed.is_ok(), "{allowed:?}");
}

#[test]
#[ignore = "manual scan: DUPLICATION_ROOTS=<project folders separated by :>"]
fn report_duplicates_of_listed_projects() {
    let roots = std::env::var("DUPLICATION_ROOTS").unwrap_or_default();
    for root in roots.split(':').filter(|root| !root.is_empty()) {
        for finding in DuplicationCheck::migration_findings(Path::new(root)) {
            println!("{root}/{}: {}", finding.source, finding.detail);
        }
    }
}
