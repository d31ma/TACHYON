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
