use super::{Model, ModelTable, Shape, TextValue, TypeText};
use serde_json::json;

fn shape(extension: &str, text: &str) -> Shape {
    TypeText::shape(extension, text).unwrap_or_else(|error| panic!("{extension} {text}: {error}"))
}

fn list(item: Shape) -> Shape {
    Shape::List(Box::new(item))
}

fn optional(inner: Shape) -> Shape {
    Shape::Optional(Box::new(inner))
}

fn model(name: &str) -> Shape {
    Shape::Model(name.to_owned())
}

#[test]
fn every_language_names_its_plain_types() {
    let cases = [
        ("ts", "string", Shape::Text),
        ("ts", "number", Shape::Number),
        ("ts", "Order[]", list(model("Order"))),
        ("ts", "Promise<Order | undefined>", optional(model("Order"))),
        ("ts", "'open' | 'closed'", Shape::Text),
        ("js", "?number", optional(Shape::Number)),
        ("js", "Array.<Order>", list(model("Order"))),
        ("py", "int", Shape::Integer),
        ("py", "list[Order]", list(model("Order"))),
        ("py", "tuple[Order, ...]", list(model("Order"))),
        ("py", "tuple[str]", list(Shape::Text)),
        ("py", "Optional[str]", optional(Shape::Text)),
        ("py", "str | None", optional(Shape::Text)),
        ("php", "?int", optional(Shape::Integer)),
        ("php", "\\Models\\Order", model("Order")),
        ("php", "array", Shape::Any),
        ("php", "list<Order>", list(model("Order"))),
        ("php", "Order[]", list(model("Order"))),
        ("php", "?array<Order>", optional(list(model("Order")))),
        ("java", "long", Shape::Integer),
        ("java", "java.util.List<Order>", list(model("Order"))),
        ("java", "Optional<String>", optional(Shape::Text)),
        ("kt", "Long?", optional(Shape::Integer)),
        ("kt", "List<Order>", list(model("Order"))),
        ("cs", "Task<List<Order>>", list(model("Order"))),
        ("cs", "decimal", Shape::Number),
        ("cs", "int?", optional(Shape::Integer)),
        ("rs", "Vec<Order>", list(model("Order"))),
        ("rs", "Option<i64>", optional(Shape::Integer)),
        ("rs", "Result<crate::models::Order, String>", model("Order")),
        ("dart", "Future<List<Order>>", list(model("Order"))),
        ("dart", "double", Shape::Number),
        ("go", "[]Order", list(model("Order"))),
        ("go", "*Order", optional(model("Order"))),
        ("go", "(Order, error)", model("Order")),
        ("go", "map[string]any", Shape::Any),
    ];
    for (extension, text, expected) in cases {
        assert_eq!(shape(extension, text), expected, "{extension} {text}");
    }
}

#[test]
fn every_language_accepts_its_native_response_as_a_return_type() {
    let cases = [
        ("ts", "Response"),
        ("java", "void"),
        ("kt", "Unit"),
        ("ts", "Promise<Response>"),
        ("js", "Response"),
        ("php", "void"),
        ("cs", "HttpResponseMessage"),
        ("cs", "Task<System.Net.Http.HttpResponseMessage>"),
        ("rs", "http::Response<Vec<u8>>"),
        ("dart", "Future<Response>"),
        ("py", "tuple[HTTPStatus, dict[str, str], str]"),
        ("py", "Tuple[int, dict[str, str], bytes]"),
    ];
    for (extension, text) in cases {
        assert_eq!(shape(extension, text), Shape::Any, "{extension} {text}");
    }
    assert_eq!(shape("go", "*http.Response"), optional(Shape::Any));
}

fn table(extension: &str, source: &str) -> ModelTable {
    let mut table = ModelTable::default();
    let models = super::model_files::ModelParser::models(extension, source);
    for (name, found) in models {
        table.models.insert(name, found);
    }
    table.join_bases();
    table
}

fn field_names(table: &ModelTable, name: &str) -> Vec<(String, bool)> {
    let Some(Model::Record(record)) = table.models.get(name) else {
        panic!("{name} is not a record: {:?}", table.models);
    };
    record
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.required))
        .collect()
}

#[test]
fn every_language_reads_its_model_fields() {
    let cases = [
        (
            "ts",
            "export interface Order {\n  id: string;\n  note?: string\n  lines: Line[]\n}\n",
        ),
        (
            "js",
            "/**\n * @typedef {Object} Order\n * @property {string} id\n * @property {string} [note]\n * @property {Line[]} lines\n */\n",
        ),
        (
            "py",
            "from dataclasses import dataclass\n\n@dataclass\nclass Order:\n    id: str\n    lines: list[Line]\n    note: str = ''\n",
        ),
        (
            "php",
            "<?php\nclass Order {\n    public string $id;\n    public array $lines;\n    public string $note = '';\n}\n",
        ),
        (
            "java",
            "class Order {\n    String id;\n    java.util.List<Line> lines;\n    String note = \"\";\n}\n",
        ),
        (
            "kt",
            "data class Order(val id: String, val lines: List<Line>, val note: String = \"\")\n",
        ),
        (
            "cs",
            "public record Order(string Id, List<Line> Lines, string? Note);\n",
        ),
        (
            "rs",
            "pub struct Order {\n    pub id: String,\n    pub lines: Vec<Line>,\n    pub note: Option<String>,\n}\n",
        ),
        (
            "dart",
            "class Order {\n  final String id;\n  final List<Line> lines;\n  final String? note;\n  const Order(this.id, this.lines, this.note);\n}\n",
        ),
        (
            "go",
            "type Order struct {\n\tID    string `json:\"id\"`\n\tLines []Line `json:\"lines\"`\n\tNote  string `json:\"note,omitempty\"`\n}\n",
        ),
    ];
    for (extension, source) in cases {
        let table = table(extension, source);
        let names = field_names(&table, "Order");
        let expected = vec![
            (String::from("id"), true),
            (String::from("lines"), true),
            (String::from("note"), false),
        ];
        let mut sorted = names.clone();
        sorted.sort();
        let mut wanted = expected.clone();
        wanted.sort();
        assert_eq!(sorted, wanted, "{extension}: {names:?}");
    }
}

#[test]
fn a_value_is_checked_against_its_model() {
    let table = table(
        "ts",
        "interface Line { sku: string; count: number }\ninterface Order { id: string; lines: Line[]; note?: string }\n",
    );
    let order = model("Order");
    let valid = json!({"id": "7", "lines": [{"sku": "a", "count": 2}]});
    assert_eq!(table.check(&order, &valid, "body"), Ok(()));
    let missing = json!({"lines": []});
    assert_eq!(
        table.check(&order, &missing, "body"),
        Err(String::from("'body.id' is required"))
    );
    let wrong = json!({"id": "7", "lines": [{"sku": "a", "count": "two"}]});
    assert_eq!(
        table.check(&order, &wrong, "body"),
        Err(String::from("'body.lines[0].count' must be a number"))
    );
    assert!(table.check_names(&model("Missing")).is_err());
}

#[test]
fn php_constructor_promoted_properties_are_model_fields() {
    let table = table(
        "php",
        "<?php\nclass Product {\n    public function __construct(public string $name, public readonly string $sku, public int $level = 1, public ?string $note = null, string $input = '', private string $secret = '') {}\n}\n",
    );
    assert_eq!(
        field_names(&table, "Product"),
        vec![
            (String::from("name"), true),
            (String::from("sku"), true),
            (String::from("level"), false),
            (String::from("note"), false),
        ]
    );
    let product = model("Product");
    assert_eq!(
        table.check(
            &product,
            &json!({"name": "Anvil", "sku": "A1", "level": 2}),
            "body"
        ),
        Ok(())
    );
    assert!(
        table
            .check(&product, &json!({"name": "Anvil"}), "body")
            .is_err()
    );
    assert!(
        table
            .check(
                &product,
                &json!({"name": "Anvil", "sku": "A1", "secret": "hidden"}),
                "body"
            )
            .is_err()
    );
    let schema = &table.model_schemas()["Product"];
    assert_eq!(schema["properties"]["level"], json!({"type": "integer"}));
    assert_eq!(schema["additionalProperties"], json!(false));
}

#[test]
fn model_records_refuse_unknown_fields_at_every_depth() {
    let table = table(
        "ts",
        "interface Line { sku: string }\ninterface Order { lines: Line[]; note?: string; metadata: any }\n",
    );
    let order = model("Order");
    let valid = json!({"lines": [{"sku": "a"}], "metadata": {"custom": true}});
    assert_eq!(table.check(&order, &valid, "body"), Ok(()));
    for (value, path) in [
        (
            json!({"lines": [], "metadata": null, "PRIVATE_SECRET": 1}),
            "body",
        ),
        (
            json!({"lines": [{"sku": "a", "PRIVATE_SECRET": 1}], "metadata": null}),
            "body.lines[0]",
        ),
    ] {
        assert_eq!(
            table.check(&order, &value, "body"),
            Err(format!(
                "'{path}' contains a field not declared in {}",
                if path == "body" { "Order" } else { "Line" }
            ))
        );
    }
    let schemas = table.model_schemas();
    for name in ["Order", "Line"] {
        assert_eq!(schemas[name]["additionalProperties"], json!(false));
    }
    assert_eq!(schemas["Order"]["properties"]["metadata"], json!({}));
}

#[test]
fn text_values_become_typed_values() {
    assert_eq!(TextValue::convert(&Shape::Integer, "42"), Some(json!(42)));
    assert_eq!(TextValue::convert(&Shape::Integer, "4.2"), None);
    assert_eq!(TextValue::convert(&Shape::Number, "4.5"), Some(json!(4.5)));
    assert_eq!(
        TextValue::convert(&Shape::Boolean, "true"),
        Some(json!(true))
    );
    assert_eq!(TextValue::convert(&Shape::Boolean, "yes"), None);
    assert_eq!(
        TextValue::convert(&optional(Shape::Text), "a"),
        Some(json!("a"))
    );
}

fn sorted_names(table: &ModelTable, name: &str) -> Vec<(String, bool)> {
    let mut names = field_names(table, name);
    names.sort();
    names
}

fn pairs(expected: &[(&str, bool)]) -> Vec<(String, bool)> {
    expected
        .iter()
        .map(|(name, required)| ((*name).to_owned(), *required))
        .collect()
}

#[test]
fn inherited_fields_join_the_record() {
    let cases = [
        (
            "ts",
            "interface Entity { id: number }\nexport interface P extends Entity { name: string }\n",
        ),
        (
            "ts",
            "interface Entity { id: number }\ntype P = { name: string } & Entity;\n",
        ),
        (
            "ts",
            "interface Entity { id: number }\ntype P = Entity & {\n  name: string\n};\n",
        ),
        (
            "py",
            "class Entity(BaseModel):\n    id: int\n\nclass P(Entity):\n    name: str\n",
        ),
        (
            "php",
            "<?php\nclass Entity {\n    public int $id;\n}\nclass P extends Entity {\n    public string $name;\n}\n",
        ),
        (
            "go",
            "type Entity struct {\n\tID int `json:\"id\"`\n}\n\ntype P struct {\n\tEntity\n\tName string `json:\"name\"`\n}\n",
        ),
        (
            "java",
            "class Entity {\n    long id;\n}\nclass P extends Entity {\n    String name;\n}\n",
        ),
        (
            "kt",
            "open class Entity(val id: Long)\ndata class P(val name: String) : Entity(1)\n",
        ),
        (
            "cs",
            "public class Entity\n{\n    public long Id { get; set; }\n}\npublic class P : Entity, IComparable\n{\n    public string Name { get; set; }\n}\n",
        ),
        (
            "dart",
            "class Entity {\n  int id = 0;\n}\nclass P extends Entity {\n  String name;\n}\n",
        ),
    ];
    for (extension, source) in cases {
        let table = table(extension, source);
        assert_eq!(
            sorted_names(&table, "P"),
            pairs(&[("id", extension != "dart"), ("name", true)]),
            "{extension}: {source}"
        );
        let p = model("P");
        assert_eq!(
            table.check(&p, &json!({"id": 1, "name": "a"}), "body"),
            Ok(()),
            "{extension}"
        );
        assert!(
            table
                .check(&p, &json!({"name": "a", "other": 1}), "body")
                .is_err(),
            "{extension}"
        );
    }
}

#[test]
fn unknown_bases_and_unread_members_leave_the_record_open() {
    let cases = [
        ("ts", "interface P extends External { name: string }\n"),
        ("ts", "interface P { [key: string]: string }\n"),
        ("py", "class P(External):\n    name: str\n"),
        (
            "php",
            "<?php\nclass P {\n    use Timestamps;\n    public string $name = '';\n}\n",
        ),
        (
            "go",
            "type P struct {\n\tpkg.External\n\tName string `json:\"name,omitempty\"`\n}\n",
        ),
        (
            "rs",
            "pub struct P {\n    pub when: chrono::DateTime<Utc>,\n}\n",
        ),
    ];
    for (extension, source) in cases {
        let table = table(extension, source);
        assert_eq!(
            table.check(&model("P"), &json!({"name": "a", "extra": true}), "body"),
            Ok(()),
            "{extension}: {source}"
        );
        assert_eq!(
            table.model_schemas()["P"].get("additionalProperties"),
            None,
            "{extension}"
        );
    }
}

#[test]
fn rust_sets_and_boxes_are_model_fields() {
    let table = table(
        "rs",
        "pub struct P {\n    pub tags: HashSet<String>,\n    pub sorted: std::collections::BTreeSet<i64>,\n    pub child: Option<Box<P>>,\n}\n",
    );
    assert_eq!(
        sorted_names(&table, "P"),
        pairs(&[("child", false), ("sorted", true), ("tags", true)])
    );
    assert_eq!(shape("rs", "Box<P>"), model("P"));
    assert_eq!(shape("rs", "HashSet<String>"), list(Shape::Text));
}

#[test]
fn arrows_and_quoted_brackets_do_not_end_the_field_list() {
    let php = table(
        "php",
        "<?php\nclass P {\n    public function __construct(public array $map = ['a' => 1, 'b' => 2], public string $open = '(', public string $close = ')', public int $after = 0) {}\n    public string $later = '';\n}\n",
    );
    assert_eq!(
        sorted_names(&php, "P"),
        pairs(&[
            ("after", false),
            ("close", false),
            ("later", false),
            ("map", false),
            ("open", false)
        ])
    );
    let ts = table("ts", "interface P { cb?: () => void; name: string }\n");
    assert_eq!(
        sorted_names(&ts, "P"),
        pairs(&[("cb", false), ("name", true)])
    );
    let multi_line = table(
        "ts",
        "interface P {\n  cb?: (value: number) => void\n  name: string\n}\n",
    );
    assert_eq!(
        sorted_names(&multi_line, "P"),
        pairs(&[("cb", false), ("name", true)])
    );
    let kotlin = table(
        "kt",
        "data class P(val label: String = \"a, b\", val count: Int)\n",
    );
    assert_eq!(
        sorted_names(&kotlin, "P"),
        pairs(&[("count", true), ("label", false)])
    );
}

#[test]
fn php_promoted_parameters_follow_php_modifier_rules() {
    let table = table(
        "php",
        "<?php\nclass P {\n    public function __construct(\n        #[SensitiveParameter] public string $a,\n        readonly public string $b,\n        final public string $c,\n        public private(set) string $d,\n        PUBLIC string $e,\n        public string &$f,\n        readonly string $g,\n        protected string $h = '',\n    ) {}\n}\n",
    );
    assert_eq!(
        sorted_names(&table, "P"),
        pairs(&[
            ("a", true),
            ("b", true),
            ("c", true),
            ("d", true),
            ("e", true),
            ("f", true),
            ("g", true)
        ])
    );
    assert_eq!(table.check_readable(&model("P")), Ok(()));
}

#[test]
fn php_models_the_binder_cannot_build_are_refused_as_request_bodies() {
    let table = table(
        "php",
        "<?php\nclass P {\n    public string $slug;\n    public function __construct(public string $name, string $raw) {\n        $this->slug = $raw;\n    }\n}\nclass Wrapper {\n    public function __construct(public array $items, public ?P $inner = null) {}\n}\n",
    );
    assert_eq!(
        sorted_names(&table, "P"),
        pairs(&[("name", true), ("slug", false)])
    );
    let refusal = table
        .check_readable(&model("Wrapper"))
        .expect_err("P needs $raw");
    assert!(refusal.contains("$raw"), "{refusal}");
    assert!(refusal.contains("Model 'P'"), "{refusal}");
}

#[test]
fn annotations_on_the_same_line_keep_their_fields() {
    let java = table(
        "java",
        "class P {\n    @JsonProperty(\"name\") private String name;\n    @Deprecated\n    int count;\n}\n",
    );
    assert_eq!(
        sorted_names(&java, "P"),
        pairs(&[("count", true), ("name", true)])
    );
    let csharp = table(
        "cs",
        "public class P\n{\n    [JsonPropertyName(\"full_name\")] public string Name { get; set; }\n    [Required]\n    public string Title { get; set; }\n    [JsonPropertyName(\"sku_code\")]\n    public string Sku { get; set; }\n}\n",
    );
    assert_eq!(
        sorted_names(&csharp, "P"),
        pairs(&[("full_name", true), ("sku_code", true), ("title", true)])
    );
    let kotlin = table(
        "kt",
        "data class P(@SerialName(\"id\") val id: String, @field:Json val n: Int)\n",
    );
    assert_eq!(
        sorted_names(&kotlin, "P"),
        pairs(&[("id", true), ("n", true)])
    );
    let rust = table(
        "rs",
        "pub struct P {\n    #[serde(default)] pub a: i32,\n}\n",
    );
    assert_eq!(sorted_names(&rust, "P"), pairs(&[("a", true)]));
}

#[test]
fn grouped_go_fields_and_dotted_jsdoc_properties_are_read_plainly() {
    let go = table("go", "type P struct {\n\tA, B string\n\tc int\n}\n");
    assert_eq!(sorted_names(&go, "P"), pairs(&[("A", true), ("B", true)]));
    let js = table(
        "js",
        "/**\n * @typedef {Object} P\n * @property {string} name\n * @property {Object} meta\n * @property {string} meta.label\n */\n",
    );
    assert_eq!(
        sorted_names(&js, "P"),
        pairs(&[("meta", true), ("name", true)])
    );
    assert_eq!(
        js.check(
            &model("P"),
            &json!({"name": "a", "meta": {"label": "b"}}),
            "body"
        ),
        Ok(())
    );
}

#[test]
fn javascript_reads_typescript_and_jsdoc_models_but_not_other_languages() {
    let root = tempfile::tempdir().expect("root");
    let models = root.path().join("server/models");
    std::fs::create_dir_all(&models).expect("models");
    std::fs::write(
        models.join("Order.ts"),
        "export interface Order {\n  sku: string\n}\n",
    )
    .expect("write");
    std::fs::write(
        models.join("Receipt.js"),
        "/**\n * @typedef {Object} Receipt\n * @property {Order} order\n */\nexport {}\n",
    )
    .expect("write");
    std::fs::write(
        models.join("Invoice.go"),
        "package models\n\ntype Invoice struct {\n\tTotal int `json:\"total\"`\n}\n",
    )
    .expect("write");
    let javascript = ModelTable::read(root.path(), "js").expect("javascript");
    assert_eq!(field_names(&javascript, "Order"), pairs(&[("sku", true)]));
    assert_eq!(
        field_names(&javascript, "Receipt"),
        pairs(&[("order", true)])
    );
    assert!(!javascript.contains("Invoice"));
    assert_eq!(javascript.check_all(), Ok(()));
    let typescript = ModelTable::read(root.path(), "ts").expect("typescript");
    assert!(typescript.contains("Order"));
    assert!(!typescript.contains("Receipt"));
}

#[test]
fn go_json_native_fields_are_open_json_values() {
    let go = table(
        "go",
        "import (\n\t\"encoding/json\"\n\t\"net/http\"\n)\n\ntype Envelope struct {\n\tBody    json.RawMessage `json:\"body\"`\n\tHeaders http.Header     `json:\"headers\"`\n}\n",
    );

    assert_eq!(go.check_all(), Ok(()));
    assert_eq!(
        go.check(
            &model("Envelope"),
            &json!({"body": {"any": [1, "two"]}, "headers": {"Accept": ["text/plain"]}}),
            "body"
        ),
        Ok(())
    );
}

#[test]
fn go_request_bound_fields_are_refused_with_the_native_request_way() {
    let go = table(
        "go",
        "import (\n\t\"context\"\n\t\"net/http\"\n)\n\ntype Call struct {\n\tRequest *http.Request\n\tContext context.Context\n}\n",
    );

    let refused = go.check_all().expect_err("request-bound fields");

    assert!(refused.contains("native request parameter"), "{refused}");
}

#[test]
fn go_request_bound_fields_left_out_of_json_are_accepted() {
    let go = table(
        "go",
        "import \"net/http\"\n\ntype Call struct {\n\tName    string\n\tRequest *http.Request `json:\"-\"`\n}\n",
    );

    assert_eq!(go.check_all(), Ok(()));
}

#[test]
fn go_types_from_other_packages_are_open_json_values() {
    for text in [
        "spec.Document",
        "time.Time",
        "json.RawMessage",
        "http.Header",
    ] {
        assert_eq!(shape("go", text), Shape::Any, "{text}");
    }
    assert_eq!(shape("go", "*spec.Document"), optional(Shape::Any));
    assert_eq!(shape("go", "[]spec.Document"), list(Shape::Any));
    assert_eq!(shape("go", "*http.Request"), optional(model("Request")));
}

#[test]
fn dart_models_without_a_constructor_are_read_by_assigning_their_fields() {
    let root = tempfile::tempdir().expect("root");
    let models = root.path().join("server/models");
    std::fs::create_dir_all(&models).expect("models");
    std::fs::write(
        models.join("Endpoint.dart"),
        "class Endpoint {\n  String profile = '';\n  final int version = 1;\n}\n",
    )
    .expect("write");
    std::fs::write(
        models.join("Settings.dart"),
        "class Settings {\n  final String profile;\n  const Settings(this.profile);\n}\n",
    )
    .expect("write");
    let text = super::Converters::program_text("dart", root.path()).expect("dart readers");
    assert!(text.contains("return Endpoint()..profile = (map['profile'] as String);"));
    assert!(!text.contains("..version"));
    assert!(text.contains("return Settings((map['profile'] as String));"));
}
