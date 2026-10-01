#![allow(clippy::expect_used)]

use super::{CompanionLanguage, KotlinMembers, Member, members, owner_table};
use std::path::Path;

struct Fixtures;

impl Fixtures {
    const PATH: &'static str = "client/services/PanelService.kt";

    fn kotlin(source: &str) -> Vec<super::KotlinMember> {
        KotlinMembers::read(Path::new(Self::PATH), source).expect("public Kotlin metadata")
    }
}

#[test]
fn kotlin_declared_returns_preserve_unit_nullable_and_nested_list_types() {
    let source = "@Service\nclass PanelService {\n  fun clear(): Unit {}\n  fun clearLater()\n  { }\n  fun increment() {}\n  fun title(): String = \"ready\"\n  fun optional(): String? = null\n  fun rows(): List<List<String?>> = emptyList()\n  fun choose(id: String, count: Int?): Boolean = true\n}\n";
    let found = Fixtures::kotlin(source);
    assert_eq!(
        found
            .iter()
            .map(|member| (member.name.as_str(), member.value_type.as_str()))
            .collect::<Vec<_>>(),
        [
            ("choose", "Boolean"),
            ("clear", "Unit"),
            ("clearLater", "Unit"),
            ("increment", "Unit"),
            ("optional", "String?"),
            ("rows", "List<List<String?>>"),
            ("title", "String")
        ]
    );
    assert_eq!(found[0].parameters, ["String", "Int?"]);
    assert!(found.iter().all(|member| member.method));
}

#[test]
fn kotlin_typed_public_fields_retain_mutability_and_exclude_owned_resources() {
    let source = "@Service\nclass PanelService {\n  val label: String = \"ready\"\n  var count: Int = 0\n  private val secret = \"private inferred value\"\n  val clock: Clock = Clock()\n  val applicationEventPublisher: ApplicationEventPublisher\n  @Needs\n  val collaborator: Port\n  @OnStart\n  fun start() {}\n  @OnStop\n  fun stop() {}\n  @OnEvent\n  fun pulse(value: Pulse) {}\n  fun answer(): String = label\n}\n";
    let found = Fixtures::kotlin(source);
    assert_eq!(
        found
            .iter()
            .map(|member| member.name.as_str())
            .collect::<Vec<_>>(),
        ["answer", "count", "label"]
    );
    assert!(found[1].mutable);
    assert!(!found[2].mutable);
    assert!(!found[2].method);
}

#[test]
fn inferred_kotlin_members_remain_runtime_valid_but_new_metadata_refuses_them() {
    for declaration in ["val label = \"ready\"", "fun answer() = \"ready\""] {
        let source = format!("@Service\nclass PanelService {{\n  {declaration}\n}}\n");
        let runtime = owner_table(
            CompanionLanguage::Kotlin,
            &source,
            Fixtures::PATH,
            &[],
            "",
            false,
        )
        .expect("unchanged inferred runtime");
        assert!(runtime.contains("tacInstance."), "{runtime}");
        let failure = KotlinMembers::read(Path::new(Fixtures::PATH), &source)
            .expect_err("explicit type required by new metadata path");
        let text = failure.to_string();
        assert!(
            text.contains("TY1405") && text.contains(Fixtures::PATH) && text.contains("explicit"),
            "{text}"
        );
    }
}

#[test]
fn kotlin_does_not_infer_a_return_from_expression_body_colons_or_literals() {
    for effects in [
        " = when (value) {",
        " = \"text: String\"",
        "",
        " :   = null",
    ] {
        assert_eq!(Member::returned_type(effects, "tac.kt"), None, "{effects}");
    }
    assert_eq!(
        Member::returned_type(" : List<String?> = rows", "tac.kt").as_deref(),
        Some("List<String?>")
    );
}

#[test]
fn swift_arrow_and_async_metadata_are_unchanged_by_kotlin_colon_support() {
    let body = "func title() -> String { return \"ready\" }\nfunc rows() async throws -> [String] { return [] }\nfunc empty() {}\n";
    let found = members(
        CompanionLanguage::Swift,
        body,
        "client/services/PanelService.swift",
    )
    .expect("Swift members");
    let title = found
        .iter()
        .find(|member| member.name == "title")
        .expect("title");
    let rows = found
        .iter()
        .find(|member| member.name == "rows")
        .expect("rows");
    let empty = found
        .iter()
        .find(|member| member.name == "empty")
        .expect("empty");
    assert_eq!(title.declared_type.as_deref(), Some("String"));
    assert_eq!(rows.declared_type.as_deref(), Some("[String]"));
    assert!(rows.asynchronous);
    assert_eq!(empty.declared_type, None);
}
