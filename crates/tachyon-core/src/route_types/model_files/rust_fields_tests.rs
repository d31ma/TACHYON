use super::rust_fields::RustFields;
use crate::route_types::{Model, ModelTable, Record, Shape};

struct RustFieldsTests;

impl RustFieldsTests {
    fn record(extension: &str, source: &str) -> Record {
        let models = ModelTable::declared_models(extension, source);
        match models.into_iter().next() {
            Some((_, Model::Record(record))) => record,
            other => panic!("Expected a model record, received {other:?}"),
        }
    }

    fn fields(body: &str) -> Vec<(String, String)> {
        RustFields::read(body, 0)
            .into_iter()
            .map(|field| (field.name, field.type_text))
            .collect()
    }

    fn names(record: &Record) -> Vec<&str> {
        record
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect()
    }
}

#[test]
fn rust_tuple_key_fields_are_not_discarded_as_methods() {
    let source = r"use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct NativeSessionProcessClaims {
    pub roots: BTreeMap<(i32, String), BTreeSet<String>>,
    pub descendants: BTreeMap<(i32, String), BTreeSet<String>>,
    pub births: BTreeMap<i32, BTreeSet<String>>,
}
";
    let record = RustFieldsTests::record("rs", source);
    assert_eq!(
        RustFieldsTests::names(&record),
        ["roots", "descendants", "births"]
    );
    assert!(record.fields.iter().all(|field| field.required));
    assert!(record.fields.iter().all(|field| field.shape == Shape::Any));
    assert!(!record.open);
}

#[test]
fn rust_raw_identifier_fields_use_the_plain_name_as_their_key() {
    let source =
        "pub struct Token {\n    pub r#type: String,\n    pub r#match: Option<String>,\n}\n";
    let record = RustFieldsTests::record("rs", source);
    assert_eq!(RustFieldsTests::names(&record), ["type", "match"]);
    let fields = RustFieldsTests::fields("pub r#type: String,");
    assert_eq!(fields, [(String::from("r#type"), String::from("String"))]);
}

#[test]
fn rust_tuple_field_parser_preserves_complete_type_text() {
    let source = "pub roots: BTreeMap<(i32, String), BTreeSet<String>>,";
    assert_eq!(
        RustFieldsTests::fields(source),
        [(
            String::from("roots"),
            String::from("BTreeMap<(i32, String), BTreeSet<String>>")
        )]
    );
}

#[test]
fn rust_fields_respect_tuple_generic_and_attribute_boundaries() {
    let source = r#"pub struct Claims {
    #[serde(default, rename = "root_claims")]
    pub roots: BTreeMap<
        (i32, String),
        BTreeSet<String>,
    >,
    pub descendants: BTreeMap<(i32, String), BTreeSet<String>>, pub births: Vec<String>,
}
"#;
    let record = RustFieldsTests::record("rs", source);
    assert_eq!(
        RustFieldsTests::names(&record),
        ["roots", "descendants", "births"]
    );
    assert_eq!(record.fields[2].shape, Shape::List(Box::new(Shape::Text)));
    assert!(!record.open);
}

#[test]
fn rust_field_visibility_parentheses_do_not_hide_names() {
    let source = r"pub struct Claims {
    pub(crate) roots: BTreeMap<(i32, String), BTreeSet<String>>,
    pub(super) descendants: Vec<String>,
    pub(self) births: i32,
    pub(in crate::claims) labels: String,
    private_count: i64,
    public_count: i64,
    pub_count: i64,
}
";
    let record = RustFieldsTests::record("rs", source);
    assert_eq!(
        RustFieldsTests::names(&record),
        [
            "roots",
            "descendants",
            "births",
            "labels",
            "private_count",
            "public_count",
            "pub_count"
        ]
    );
    assert!(record.fields.iter().all(|field| field.required));
    assert!(!record.open);
}

#[test]
fn rust_method_parameters_and_return_types_are_not_fields() {
    let methods = [
        "pub fn roots(value: i32) -> String { String::new() }",
        "fn descendants(&self, value: (i32, String)) -> i64 { 0 }",
        "pub(crate) async fn births(value: i32) -> i64 { 0 }",
        "unsafe extern \"C\" fn call(value: i32) -> i64 { 0 }",
        "pub const fn count(&self) -> i64 { 0 }",
        "fn generic<T: Clone>(value: T) -> T { value }",
        "pub fn declared(value: i32) -> i64;",
    ];
    for method in methods {
        assert!(RustFieldsTests::fields(method).is_empty(), "{method}");
    }
}

#[test]
fn rust_named_function_pointer_and_unit_types_are_fields_even_without_wire_support() {
    assert_eq!(
        RustFieldsTests::fields("pub callback: fn(i32) -> i64, pub empty: (),"),
        [
            (String::from("callback"), String::from("fn(i32) -> i64")),
            (String::from("empty"), String::from("()")),
        ]
    );
}

#[test]
fn rust_unnamed_and_incomplete_members_are_not_named_fields() {
    for member in [
        "",
        "pub",
        "pub(crate)",
        "pub missing:",
        ": String",
        "pub (i32, String)",
        "use crate::Claims;",
    ] {
        assert!(RustFieldsTests::fields(member).is_empty(), "{member}");
    }
}

#[test]
fn other_braced_languages_keep_fields_and_exclude_methods() {
    let cases = [
        (
            "java",
            "class P {\n private String name;\n public String getName() { return name; }\n}\n",
        ),
        (
            "cs",
            "class P {\n public string Name { get; set; }\n public string GetName() { return Name; }\n}\n",
        ),
        (
            "kt",
            "data class P(val name: String) {\n fun read(value: Int): String { return name }\n}\n",
        ),
        (
            "swift",
            "struct P {\n var name: String\n func read(value: Int) -> String { return name }\n}\n",
        ),
        (
            "dart",
            "class P {\n final String name;\n P(this.name);\n String read(int value) { return name; }\n}\n",
        ),
        (
            "php",
            "<?php\nclass P {\n public function __construct(public string $name) {}\n public function read(int $value): string { return $this->name; }\n}\n",
        ),
    ];
    for (extension, source) in cases {
        let record = RustFieldsTests::record(extension, source);
        assert_eq!(RustFieldsTests::names(&record), ["name"], "{extension}");
    }
}

#[test]
fn rust_raw_model_field_spans_reference_original_source_bytes() {
    let source = r"// Claims may contain tuple keys.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Claims {
    #[serde(default)]
    pub(in crate::claims) roots: BTreeMap<(i32, String), BTreeSet<String>>,
    // A second field keeps the original offset after comment masking.
    pub labels: Vec</* annotation */ String>,
}
";
    let models = ModelTable::rust_model_fields(source);
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].0, "Claims");
    let fields = &models[0].1;
    assert_eq!(fields.len(), 2);
    assert_eq!(&source[fields[0].name_range.clone()], "roots");
    assert_eq!(
        &source[fields[0].type_range.clone()],
        "BTreeMap<(i32, String), BTreeSet<String>>"
    );
    assert_eq!(
        fields[0].type_text,
        "BTreeMap<(i32, String), BTreeSet<String>>"
    );
    assert_eq!(&source[fields[1].name_range.clone()], "labels");
    assert_eq!(
        &source[fields[1].type_range.clone()],
        "Vec</* annotation */ String>"
    );
    assert!(!fields[1].type_text.contains("annotation"));
    assert!(fields[1].type_text.ends_with("String>"));
}

#[test]
fn rust_alias_enum_and_unnamed_tuple_models_keep_existing_behavior() {
    let source = "pub enum Mode { Active, Quiet }\npub type Names = Vec<String>;\npub struct Pair(i32, String);\npub struct Named { pub label: String }\n";
    let declared = ModelTable::declared_models("rs", source);
    assert!(
        declared
            .iter()
            .any(|(name, model)| name == "Mode" && *model == Model::Alias(Shape::Text))
    );
    assert!(declared.iter().any(|(name, model)| name == "Names"
        && *model == Model::Alias(Shape::List(Box::new(Shape::Text)))));
    let fields = ModelTable::rust_model_fields(source);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].0, "Named");
    assert_eq!(fields[0].1[0].name, "label");
}
