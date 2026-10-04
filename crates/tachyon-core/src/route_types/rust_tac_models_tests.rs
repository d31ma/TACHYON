use super::{Converters, RustTacModels};
use crate::Failure;
use std::path::Path;

struct RustTacModelsTests;

impl RustTacModelsTests {
    const MODEL_PATH: &'static str = "client/models/Claims.rs";
    const SERVICE_PATH: &'static str = "client/services/ProbeService.rs";
    const INTERNAL: &'static str = "use std::collections::{BTreeMap, BTreeSet};\n#[derive(Default)]\npub struct Claims {\n    pub roots: BTreeMap<(i32, String), BTreeSet<String>>,\n    pub births: BTreeMap<i32, BTreeSet<String>>,\n}\n";
    const LEAF: &'static str =
        "#[derive(Clone, Default)]\npub struct Leaf {\n    pub label: String,\n}\n";
    const TREE: &'static str =
        "#[derive(Clone, Default)]\npub struct Tree {\n    pub leaves: Vec<Option<Leaf>>,\n}\n";

    fn catalog(source: &str) -> RustTacModels {
        RustTacModels::of(&[(Self::MODEL_PATH.to_owned(), source.to_owned())])
    }

    fn require(catalog: &RustTacModels, kind: &str) -> Result<(), Failure> {
        catalog.require(&[kind.to_owned()], Path::new(Self::SERVICE_PATH))
    }

    fn refusal(result: Result<(), Failure>) -> Failure {
        match result {
            Err(failure) => failure,
            Ok(()) => panic!("An unsupported Rust wire type was accepted"),
        }
    }

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap_or_else(|failure| panic!("project: {failure}"));
        Self::write(root.path(), Self::MODEL_PATH, Self::INTERNAL);
        root
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap_or(root))
            .unwrap_or_else(|failure| panic!("fixture directory: {failure}"));
        std::fs::write(path, source).unwrap_or_else(|failure| panic!("fixture source: {failure}"));
    }

    fn located(failure: &Failure, path: &str, source: &str, text: &str) {
        let start = source
            .find(text)
            .unwrap_or_else(|| panic!("missing field type: {text}"));
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.number(), 1405);
        assert_eq!(diagnostic.spans[0].file, path);
        assert_eq!(diagnostic.spans[0].start, start as u64);
        assert_eq!(diagnostic.spans[0].end, (start + text.len()) as u64);
        assert!(diagnostic.spans.len() >= 2);
    }

    fn owner(source: &str, excluded: &[String]) -> Result<(), Failure> {
        let root = Self::project();
        Converters::require_rust_owner(root.path(), source, Path::new(Self::SERVICE_PATH), excluded)
    }

    fn event_module(source: &str) -> crate::events::EventModule {
        crate::events::extract(Path::new(Self::SERVICE_PATH), source, "tac")
            .unwrap_or_else(|failure| panic!("events: {failure}"))
            .unwrap_or_else(|| panic!("The fixture has no declared events"))
    }

    fn mapping(root: &Path, source: &str) -> String {
        let path = Path::new("client/converters/LeafConverter.rs");
        let rewritten = crate::declared_contract::ConverterModelTable::new(root, "tac", "rs")
            .written(path, source.to_owned())
            .unwrap_or_else(|failure| panic!("mapping metadata: {failure}"));
        crate::declared_contract::DeclaredContract::concrete(path, &rewritten, "tac")
            .unwrap_or_else(|| panic!("The converter was not concretized"))
    }
}

#[test]
fn internal_rust_map_set_tuple_models_keep_declarations_without_wire_codecs() {
    let root = RustTacModelsTests::project();
    let models = crate::model_sources::ModelSources::read(root.path(), "client", "rs")
        .unwrap_or_else(|failure| panic!("models: {failure}"));
    let authored = models.rust_module();
    assert!(authored.contains("pub struct Claims"));
    assert!(authored.contains("BTreeMap<(i32, String), BTreeSet<String>>"));
    assert!(authored.contains("BTreeMap<i32, BTreeSet<String>>"));
    let codecs = Converters::tac_text("rs", root.path())
        .unwrap_or_else(|failure| panic!("codecs: {failure}"));
    assert!(!codecs.contains("impl TacModel for crate::models::Claims"));
}

#[test]
fn supported_rust_models_keep_complete_transitive_vec_option_codecs() {
    let catalog = RustTacModels::of(&[
        (
            "client/models/Leaf.rs".to_owned(),
            RustTacModelsTests::LEAF.to_owned(),
        ),
        (
            "client/models/Tree.rs".to_owned(),
            RustTacModelsTests::TREE.to_owned(),
        ),
        (
            RustTacModelsTests::MODEL_PATH.to_owned(),
            RustTacModelsTests::INTERNAL.to_owned(),
        ),
    ]);
    RustTacModelsTests::require(&catalog, "Vec<Option<Tree>>")
        .unwrap_or_else(|failure| panic!("supported wire closure: {failure}"));
    let codecs = catalog.text();
    assert!(codecs.contains("impl TacModel for crate::models::Leaf"));
    assert!(codecs.contains("impl TacModel for crate::models::Tree"));
    assert!(codecs.contains("leaves: tac_read_field(value, \"leaves\")"));
    assert!(!codecs.contains("impl TacModel for crate::models::Claims"));
}

#[test]
fn raw_identifier_fields_keep_their_rust_name_and_use_the_plain_wire_key() {
    let source = "#[derive(Clone, Default)]\npub struct Token {\n    pub r#type: String,\n    pub r#match: Option<String>,\n    pub label: String,\n}\n";
    let catalog = RustTacModelsTests::catalog(source);
    RustTacModelsTests::require(&catalog, "Token")
        .unwrap_or_else(|failure| panic!("raw identifier model: {failure}"));
    let codecs = catalog.text();
    assert!(
        codecs.contains("r#type: tac_read_field(value, \"type\")"),
        "{codecs}"
    );
    assert!(
        codecs.contains("r#match: tac_read_field(value, \"match\")"),
        "{codecs}"
    );
    assert!(codecs.contains("(String::from(\"type\"), TacModel::write(self.r#type))"));
    assert!(codecs.contains("(String::from(\"match\"), TacModel::write(self.r#match))"));
    assert!(codecs.contains("label: tac_read_field(value, \"label\")"));
}

#[test]
fn a_rust_wire_model_refuses_map_set_tuple_fields_at_the_original_field_type() {
    let catalog = RustTacModelsTests::catalog(RustTacModelsTests::INTERNAL);
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, "Claims"));
    RustTacModelsTests::located(
        &failure,
        RustTacModelsTests::MODEL_PATH,
        RustTacModelsTests::INTERNAL,
        "BTreeMap<(i32, String), BTreeSet<String>>",
    );
}

#[test]
fn a_set_field_never_inherits_the_schema_list_codec() {
    let source = "pub struct Claims {\n    pub names: BTreeSet<String>,\n}\n";
    let catalog = RustTacModelsTests::catalog(source);
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, "Claims"));
    RustTacModelsTests::located(
        &failure,
        RustTacModelsTests::MODEL_PATH,
        source,
        "BTreeSet<String>",
    );
    assert!(!catalog.text().contains("impl TacModel"));
}

#[test]
fn a_tuple_field_never_disappears_from_wire_eligibility() {
    let source = "pub struct Claims {\n    pub pair: (i32, String),\n}\n";
    let catalog = RustTacModelsTests::catalog(source);
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, "Claims"));
    RustTacModelsTests::located(
        &failure,
        RustTacModelsTests::MODEL_PATH,
        source,
        "(i32, String)",
    );
    assert!(!catalog.text().contains("impl TacModel"));
}

#[test]
fn a_transitive_wire_model_reports_the_inner_unsupported_model_field() {
    let wrapper = "pub struct Wrapper {\n    pub claims: Option<Claims>,\n}\n";
    let catalog = RustTacModels::of(&[
        ("client/models/Wrapper.rs".to_owned(), wrapper.to_owned()),
        (
            RustTacModelsTests::MODEL_PATH.to_owned(),
            RustTacModelsTests::INTERNAL.to_owned(),
        ),
    ]);
    let failure =
        RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, "Vec<Wrapper>"));
    RustTacModelsTests::located(
        &failure,
        RustTacModelsTests::MODEL_PATH,
        RustTacModelsTests::INTERNAL,
        "BTreeMap<(i32, String), BTreeSet<String>>",
    );
    assert!(!catalog.text().contains("impl TacModel"));
}

#[test]
fn rust_scalar_and_vec_option_aliases_keep_existing_native_trait_eligibility() {
    let catalog = RustTacModels::of(&[
        (
            "client/models/Count.rs".to_owned(),
            "pub type Count = i64;\n".to_owned(),
        ),
        (
            "client/models/Names.rs".to_owned(),
            "pub type Names = Vec<Option<String>>;\n".to_owned(),
        ),
        (
            "client/models/Totals.rs".to_owned(),
            "pub struct Totals {\n    pub count: Count,\n    pub names: Names,\n}\n".to_owned(),
        ),
    ]);
    RustTacModelsTests::require(&catalog, "Totals")
        .unwrap_or_else(|failure| panic!("aliases: {failure}"));
    assert!(
        catalog
            .text()
            .contains("impl TacModel for crate::models::Totals")
    );
    assert!(
        !catalog
            .text()
            .contains("impl TacModel for crate::models::Count")
    );
    assert!(
        !catalog
            .text()
            .contains("impl TacModel for crate::models::Names")
    );
}

#[test]
fn unknown_nested_rust_models_refuse_before_generating_a_partial_codec() {
    let source = "pub struct Claims {\n    pub child: Vec<Option<Missing>>,\n}\n";
    let catalog = RustTacModelsTests::catalog(source);
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, "Claims"));
    RustTacModelsTests::located(
        &failure,
        RustTacModelsTests::MODEL_PATH,
        source,
        "Vec<Option<Missing>>",
    );
    assert!(!catalog.text().contains("impl TacModel"));
}

#[test]
fn recursive_rust_models_retain_the_supported_vec_closure() {
    let source = "pub struct Claims {\n    pub children: Vec<Claims>,\n}\n";
    let catalog = RustTacModelsTests::catalog(source);
    RustTacModelsTests::require(&catalog, "Claims")
        .unwrap_or_else(|failure| panic!("recursive Vec: {failure}"));
    assert!(
        catalog
            .text()
            .contains("children: tac_read_field(value, \"children\")")
    );
}

#[test]
fn a_page_field_requiring_an_internal_map_model_is_located_before_table_emission() {
    let source = "#[Service]\nstruct ProbeService {\n    claims: Claims,\n}\nimpl ProbeService {\n    fn new() -> Self { unimplemented!() }\n}\n";
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::owner(source, &[]));
    RustTacModelsTests::located(
        &failure,
        RustTacModelsTests::MODEL_PATH,
        RustTacModelsTests::INTERNAL,
        "BTreeMap<(i32, String), BTreeSet<String>>",
    );
}

#[test]
fn a_page_argument_requiring_an_internal_map_model_is_located_before_table_emission() {
    let source = "#[Service]\nstruct ProbeService {\n    count: i64,\n}\nimpl ProbeService {\n    fn new() -> Self { Self { count: 0 } }\n    fn accept(&self, value: Claims) -> String { String::new() }\n}\n";
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::owner(source, &[]));
    RustTacModelsTests::located(
        &failure,
        RustTacModelsTests::MODEL_PATH,
        RustTacModelsTests::INTERNAL,
        "BTreeMap<(i32, String), BTreeSet<String>>",
    );
}

#[test]
fn a_page_result_requiring_an_internal_map_model_is_located_before_table_emission() {
    let source = "#[Service]\nstruct ProbeService {\n    count: i64,\n}\nimpl ProbeService {\n    fn new() -> Self { Self { count: 0 } }\n    fn read(&self) -> Claims { unimplemented!() }\n}\n";
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::owner(source, &[]));
    RustTacModelsTests::located(
        &failure,
        RustTacModelsTests::MODEL_PATH,
        RustTacModelsTests::INTERNAL,
        "BTreeMap<(i32, String), BTreeSet<String>>",
    );
}

#[test]
fn an_excluded_rust_event_member_does_not_become_a_page_codec_root() {
    let source = "#[Service]\nstruct ProbeService {\n    count: i64,\n}\nimpl ProbeService {\n    fn new() -> Self { Self { count: 0 } }\n    fn read(&self) -> Claims { unimplemented!() }\n}\n";
    RustTacModelsTests::owner(source, &["read".to_owned()])
        .unwrap_or_else(|failure| panic!("excluded member: {failure}"));
}

#[test]
fn direct_internal_collection_fields_keep_the_existing_rust_page_selection() {
    let source = "#[Service]\nstruct ProbeService {\n    count: i64,\n    claims: std::collections::BTreeMap<i32, String>,\n}\nimpl ProbeService {\n    fn new() -> Self { unimplemented!() }\n    fn read(&self) -> String { String::new() }\n}\n";
    RustTacModelsTests::owner(source, &[])
        .unwrap_or_else(|failure| panic!("internal selected owner: {failure}"));
}

#[test]
fn a_declared_rust_event_payload_with_internal_maps_is_located_before_generation() {
    let root = RustTacModelsTests::project();
    let source = "#[Service]\nstruct ProbeService {\n    count: i64,\n}\nimpl ProbeService {\n    #[OnEvent]\n    fn receive(&mut self, claims: Claims) {}\n}\n";
    let module = RustTacModelsTests::event_module(source);
    let failure = RustTacModelsTests::refusal(Converters::require_rust_events(
        root.path(),
        Path::new(RustTacModelsTests::SERVICE_PATH),
        Some(&module),
    ));
    RustTacModelsTests::located(
        &failure,
        RustTacModelsTests::MODEL_PATH,
        RustTacModelsTests::INTERNAL,
        "BTreeMap<(i32, String), BTreeSet<String>>",
    );
}

#[test]
fn a_typed_rust_converter_cannot_project_a_vec_schema_into_a_set_codec() {
    let root = RustTacModelsTests::project();
    RustTacModelsTests::write(
        root.path(),
        "client/models/Leaf.rs",
        "pub struct Leaf {\n    pub tags: Vec<String>,\n}\n",
    );
    let result = "use std::collections::BTreeSet;\npub struct LeafSummary {\n    pub tags: BTreeSet<String>,\n}\n";
    RustTacModelsTests::write(root.path(), "client/models/LeafSummary.rs", result);
    let source = RustTacModelsTests::mapping(
        root.path(),
        "#[Converter]\ntrait LeafConverter {\n    fn map(&self, leaf: Leaf) -> BehaviorResult<LeafSummary>;\n}\n",
    );
    let failure = RustTacModelsTests::refusal(Converters::require_rust_mappings(
        root.path(),
        Path::new("client/converters/LeafConverter.rs"),
        &source,
    ));
    RustTacModelsTests::located(
        &failure,
        "client/models/LeafSummary.rs",
        result,
        "BTreeSet<String>",
    );
}

#[test]
fn an_unregistered_rust_type_cannot_borrow_a_longer_enums_codec() {
    let catalog = RustTacModels::of(&[(
        "client/models/StatusLong.rs".to_owned(),
        "pub enum StatusLong { Ready, Done }\n".to_owned(),
    )]);
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, "Status"));
    assert_eq!(failure.diagnostics()[0].code.number(), 1405);
    assert_eq!(
        failure.diagnostics()[0].spans[0].file,
        RustTacModelsTests::SERVICE_PATH
    );
    RustTacModelsTests::require(&catalog, "StatusLong")
        .unwrap_or_else(|failure| panic!("registered enum: {failure}"));
}

#[test]
fn a_supported_rust_type_alias_cannot_become_a_similarly_named_enumeration() {
    let catalog = RustTacModels::of(&[
        (
            "client/models/StatusLong.rs".to_owned(),
            "pub enum StatusLong { Ready }\n".to_owned(),
        ),
        (
            "client/models/Status.rs".to_owned(),
            "pub type Status = String;\n".to_owned(),
        ),
    ]);
    RustTacModelsTests::require(&catalog, "Status")
        .unwrap_or_else(|failure| panic!("scalar alias: {failure}"));
    let codecs = catalog.text();
    assert!(codecs.contains("impl TacModel for crate::models::StatusLong"));
    assert!(!codecs.contains("impl TacModel for crate::models::Status {"));
}

#[test]
fn deeply_nested_rust_container_types_receive_a_bounded_refusal() {
    let source = format!(
        "pub struct Claims {{\n    pub items: {}String{},\n}}\n",
        "Vec<".repeat(33),
        ">".repeat(33)
    );
    let catalog = RustTacModelsTests::catalog(&source);
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, "Claims"));
    assert_eq!(failure.diagnostics()[0].code.number(), 1405);
    assert_eq!(
        failure.diagnostics()[0].spans[0].file,
        RustTacModelsTests::MODEL_PATH
    );
    assert!(!catalog.text().contains("impl TacModel"));
}

#[test]
fn qualified_existing_rust_primitive_traits_keep_wire_eligibility() {
    let source = "pub struct Claims {\n    pub count: std::primitive::i64,\n    pub label: core::option::Option<std::string::String>,\n}\n";
    let catalog = RustTacModelsTests::catalog(source);
    RustTacModelsTests::require(&catalog, "Claims")
        .unwrap_or_else(|failure| panic!("qualified native traits: {failure}"));
    assert!(
        catalog
            .text()
            .contains("count: tac_read_field(value, \"count\")")
    );
    assert!(
        catalog
            .text()
            .contains("label: tac_read_field(value, \"label\")")
    );
}

#[test]
fn recursive_vec_alias_roots_share_the_supported_record_closure() {
    let catalog = RustTacModels::of(&[
        (
            "client/models/Nodes.rs".to_owned(),
            "pub type Nodes = Vec<Node>;\n".to_owned(),
        ),
        (
            "client/models/Node.rs".to_owned(),
            "pub struct Node {\n    pub children: Nodes,\n}\n".to_owned(),
        ),
    ]);
    for kind in ["Node", "Nodes", "Vec<Option<Nodes>>"] {
        RustTacModelsTests::require(&catalog, kind)
            .unwrap_or_else(|failure| panic!("recursive alias root {kind}: {failure}"));
    }
    let codecs = catalog.text();
    assert!(codecs.contains("impl TacModel for crate::models::Node {"));
    assert!(!codecs.contains("impl TacModel for crate::models::Nodes {"));
}

#[test]
fn pure_rust_alias_cycles_remain_bounded_refusals() {
    let catalog = RustTacModels::of(&[
        (
            "client/models/First.rs".to_owned(),
            "pub type First = Second;\n".to_owned(),
        ),
        (
            "client/models/Second.rs".to_owned(),
            "pub type Second = First;\n".to_owned(),
        ),
    ]);
    for kind in ["First", "Second", "Vec<First>"] {
        let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, kind));
        assert_eq!(failure.diagnostics()[0].code.number(), 1405);
    }
    assert!(!catalog.text().contains("impl TacModel"));
}

#[test]
fn captured_scalar_or_supplied_event_names_do_not_bypass_unsupported_fields() {
    let field_type = "std::collections::BTreeMap<i32, bool>";
    for name in ["String", "ConnectivityChanged"] {
        let path = format!("client/models/{name}.rs");
        let source = format!("pub struct {name} {{\n    pub claims: {field_type},\n}}\n");
        let catalog = RustTacModels::of(&[(path.clone(), source.clone())]);
        for kind in [
            name.to_owned(),
            format!("crate::models::{name}"),
            format!("::crate::models::{name}"),
        ] {
            let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, &kind));
            RustTacModelsTests::located(&failure, &path, &source, field_type);
        }
        assert!(
            !catalog
                .text()
                .contains(&format!("impl TacModel for crate::models::{name} {{"))
        );
    }
}

#[test]
fn qualified_real_primitives_do_not_resolve_to_a_captured_string_record() {
    let source = "pub struct String {\n    pub claims: std::collections::BTreeMap<i32, bool>,\n}\n";
    let catalog = RustTacModels::of(&[("client/models/String.rs".to_owned(), source.to_owned())]);
    for kind in [
        "std::string::String",
        "::std::string::String",
        "std::primitive::i64",
        "::core::primitive::bool",
        "Vec<Option<std::string::String>>",
    ] {
        RustTacModelsTests::require(&catalog, kind)
            .unwrap_or_else(|failure| panic!("actual qualified primitive {kind}: {failure}"));
    }
    assert!(
        !catalog
            .text()
            .contains("impl TacModel for crate::models::String {")
    );
}

#[test]
fn crate_root_supplied_events_keep_their_actual_namespace_when_shadowed() {
    let source = "pub struct ConnectivityChanged {\n    pub claims: std::collections::BTreeMap<i32, bool>,\n}\n";
    let catalog = RustTacModels::of(&[(
        "client/models/ConnectivityChanged.rs".to_owned(),
        source.to_owned(),
    )]);
    for kind in [
        "crate::ConnectivityChanged",
        "::crate::ConnectivityChanged",
        "Vec<Option<crate::ConnectivityChanged>>",
    ] {
        RustTacModelsTests::require(&catalog, kind)
            .unwrap_or_else(|failure| panic!("actual supplied event {kind}: {failure}"));
    }
    for kind in [
        "other::ConnectivityChanged",
        "std::ConnectivityChanged",
        "crate::models::Missing",
    ] {
        let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, kind));
        assert_eq!(failure.diagnostics()[0].code.number(), 1405);
    }
    let unshadowed = RustTacModels::of(&[]);
    RustTacModelsTests::require(&unshadowed, "ConnectivityChanged")
        .unwrap_or_else(|failure| panic!("unshadowed supplied event: {failure}"));
}

#[test]
fn multiline_supported_aliases_keep_complete_vec_option_targets() {
    let catalog = RustTacModels::of(&[
        ("client/models/Names.rs".to_owned(), "pub type Names =\n    Vec<Option<std::string::String>>;\n".to_owned()),
        ("client/models/Labels.rs".to_owned(), "pub type Labels\n    = Vec<\n        Option<\n            String\n        >\n    >;\n".to_owned()),
        ("client/models/Annotated.rs".to_owned(), "pub type Annotated =\n    Vec</* actual item */ Option<String>>;\n".to_owned()),
        ("client/models/Totals.rs".to_owned(), "pub struct Totals {\n    pub names: Names,\n    pub labels: Labels,\n    pub annotated: Annotated,\n}\n".to_owned()),
    ]);
    for kind in ["Names", "Labels", "Annotated", "Totals"] {
        RustTacModelsTests::require(&catalog, kind)
            .unwrap_or_else(|failure| panic!("multiline alias {kind}: {failure}"));
    }
    let codecs = catalog.text();
    assert!(codecs.contains("impl TacModel for crate::models::Totals {"));
    assert!(codecs.contains("names: tac_read_field(value, \"names\")"));
    assert!(codecs.contains("labels: tac_read_field(value, \"labels\")"));
    assert!(codecs.contains("annotated: tac_read_field(value, \"annotated\")"));
}

#[test]
fn multiline_aliases_do_not_make_unsupported_map_targets_wire_eligible() {
    let source = "pub struct Claims {\n    pub index: ClaimIndex,\n}\n";
    let catalog = RustTacModels::of(&[
        (
            "client/models/ClaimIndex.rs".to_owned(),
            "pub type ClaimIndex =\n    std::collections::BTreeMap<i32, bool>;\n".to_owned(),
        ),
        (RustTacModelsTests::MODEL_PATH.to_owned(), source.to_owned()),
    ]);
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, "Claims"));
    RustTacModelsTests::located(
        &failure,
        RustTacModelsTests::MODEL_PATH,
        source,
        "ClaimIndex",
    );
    assert!(!catalog.text().contains("impl TacModel"));
}

#[test]
fn empty_enums_are_refused_without_partial_wire_codecs() {
    let path = "client/models/Never.rs";
    let source = "pub enum Never {}\n";
    let catalog = RustTacModels::of(&[(path.to_owned(), source.to_owned())]);
    for kind in ["Never", "Vec<Never>", "Option<Never>"] {
        let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, kind));
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.number(), 1405);
        assert_eq!(diagnostic.spans[0].file, path);
        assert_eq!(diagnostic.spans[0].start, 9);
        assert_eq!(diagnostic.spans[0].end, 14);
    }
    assert!(!catalog.text().contains("impl TacModel"));
}

#[test]
fn nonempty_plain_enums_remain_supported_alongside_internal_empty_enums() {
    let catalog = RustTacModels::of(&[
        (
            "client/models/Never.rs".to_owned(),
            "pub enum Never {}\n".to_owned(),
        ),
        (
            "client/models/Phase.rs".to_owned(),
            "pub enum Phase { Waiting, Ready }\n".to_owned(),
        ),
    ]);
    RustTacModelsTests::require(&catalog, "Vec<Option<Phase>>")
        .unwrap_or_else(|failure| panic!("nonempty plain enum: {failure}"));
    let codecs = catalog.text();
    assert!(codecs.contains("impl TacModel for crate::models::Phase {"));
    assert!(!codecs.contains("impl TacModel for crate::models::Never {"));
}

#[test]
fn a_recursive_alias_cannot_hide_an_unsupported_field_in_its_captured_record() {
    let source = "pub struct Node {\n    pub children: Nodes,\n    pub claims: std::collections::BTreeMap<i32, bool>,\n}\n";
    let catalog = RustTacModels::of(&[
        (
            "client/models/Nodes.rs".to_owned(),
            "pub type Nodes = Vec<Node>;\n".to_owned(),
        ),
        ("client/models/Node.rs".to_owned(), source.to_owned()),
    ]);
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, "Nodes"));
    RustTacModelsTests::located(
        &failure,
        "client/models/Node.rs",
        source,
        "std::collections::BTreeMap<i32, bool>",
    );
    assert!(!catalog.text().contains("impl TacModel"));
}

#[test]
fn crossing_a_captured_record_does_not_reset_the_alias_declaration_budget() {
    let mut files = Vec::new();
    for index in 0..128 {
        let next = match index {
            127 => "Node".to_owned(),
            _ => format!("First{}", index + 1),
        };
        files.push((
            format!("client/models/First{index}.rs"),
            format!("pub type First{index} = {next};\n"),
        ));
    }
    files.push((
        "client/models/Node.rs".to_owned(),
        "pub struct Node {\n    pub value: Second0,\n}\n".to_owned(),
    ));
    for index in 0..128 {
        let next = match index {
            127 => "String".to_owned(),
            _ => format!("Second{}", index + 1),
        };
        files.push((
            format!("client/models/Second{index}.rs"),
            format!("pub type Second{index} = {next};\n"),
        ));
    }
    let catalog = RustTacModels::of(&files);
    let failure = RustTacModelsTests::refusal(RustTacModelsTests::require(&catalog, "First0"));
    assert_eq!(failure.diagnostics()[0].code.number(), 1405);
    assert!(
        failure
            .to_string()
            .contains("model closure exceeds 256 declarations")
    );
}
