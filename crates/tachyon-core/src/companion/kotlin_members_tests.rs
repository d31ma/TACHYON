#![allow(clippy::expect_used)]

use super::{
    CompanionLanguage, KotlinMembers, Member, members, owner_table, with_required_constructor,
};
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
fn kotlin_lateinit_inputs_are_writable_and_private_fields_stay_hidden() {
    let source = "@Service\nclass PanelService {\n lateinit var route: String\n public lateinit var catalogue: List<String>\n lateinit public var label: String\n private lateinit var first: String\n lateinit private var second: String\n}\n";
    let found = Fixtures::kotlin(source);
    assert_eq!(
        found
            .iter()
            .map(|member| (member.name.as_str(), member.value_type.as_str()))
            .collect::<Vec<_>>(),
        [
            ("catalogue", "List<String>"),
            ("label", "String"),
            ("route", "String")
        ]
    );
    assert!(found.iter().all(|member| member.mutable && !member.method));
    let runtime = owner_table(
        CompanionLanguage::Kotlin,
        source,
        Fixtures::PATH,
        &[],
        "",
        false,
    )
    .expect("lateinit runtime fields");
    assert!(runtime.contains("tacInstance.route ="), "{runtime}");
    assert!(!runtime.contains("tacInstance.first"), "{runtime}");
    assert!(!runtime.contains("tacInstance.second"), "{runtime}");
}

#[test]
fn kotlin_required_inputs_use_existing_closed_model_readers() {
    let source = "@Service\n@NeedsAll\nclass PanelService {\n lateinit var guide: Guide\n val guides: List<Guide?>\n val optional: Guide?\n @Needs\n private val documentTitle: DocumentTitle\n}\n";
    let generated =
        with_required_constructor(CompanionLanguage::Kotlin, source, Fixtures::PATH, true);
    for (field, kind) in [
        ("guide", "Guide"),
        ("guides", "List<Guide?>"),
        ("optional", "Guide?"),
    ] {
        assert!(
            generated.contains(&format!(
                "this.{field} = tacRead<{kind}>(tacRequiredValue(tacProps, \"{field}\"))"
            )),
            "{generated}"
        );
    }
    assert!(
        generated.contains("this.documentTitle = tacPage.documentTitle"),
        "{generated}"
    );
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

#[test]
fn kotlin_typed_getters_keep_nullable_generic_types_and_field_mutability() {
    let source = "@Service\nclass PanelService {\n val id: String get() = \"routing\"\n val rows: List<List<String?>> get() = emptyList()\n var note: String? = null\n   get() = field\n   set(value) { field = value }\n val later: String?\n   get() = null\n}\n";
    let found = Fixtures::kotlin(source);
    assert_eq!(
        found
            .iter()
            .map(|member| (
                member.name.as_str(),
                member.value_type.as_str(),
                member.mutable
            ))
            .collect::<Vec<_>>(),
        [
            ("id", "String", false),
            ("later", "String?", false),
            ("note", "String?", true),
            ("rows", "List<List<String?>>", false),
        ]
    );
    let runtime = owner_table(
        CompanionLanguage::Kotlin,
        source,
        Fixtures::PATH,
        &[],
        "",
        false,
    )
    .expect("existing Kotlin getter runtime");
    assert!(runtime.contains("tacInstance.id"), "{runtime}");
    assert!(runtime.contains("tacInstance.note ="), "{runtime}");
    assert!(!runtime.contains("tacRead<String? get"), "{runtime}");
}

#[test]
fn kotlin_getter_metadata_never_infers_the_expression_return_type() {
    for declaration in ["val answer get() = 42", "val answer get() = \"String\""] {
        let source = format!("@Service\nclass PanelService {{\n {declaration}\n}}\n");
        let text = KotlinMembers::read(Path::new(Fixtures::PATH), &source)
            .expect_err("no inferred getter types")
            .to_string();
        assert!(
            text.contains("TY1405") && text.contains(Fixtures::PATH),
            "{text}"
        );
    }
    assert_eq!(
        Member::property_type(": GetterValue = value", "tac.kt").as_deref(),
        Some("GetterValue")
    );
    assert_eq!(
        Member::property_type(": List<GetterValue> = rows", "tac.kt").as_deref(),
        Some("List<GetterValue>")
    );
    assert_eq!(
        Member::property_type(": String { return \"swift\" }", "tac.swift").as_deref(),
        Some("String { return \"swift\" }")
    );
}
