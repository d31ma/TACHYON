use super::ModelTable;

fn table(extension: &str, source: &str) -> ModelTable {
    let file = format!("server/models/Model.{extension}");
    ModelTable::from_files(&[(file, source.to_owned())])
}

fn assert_names(extension: &str, source: &str, names: &[&str]) {
    let read = ModelTable::declared_names(extension, source);
    for name in names {
        assert!(
            read.iter().any(|found| found == name),
            "{extension}: {name} in {read:?}"
        );
    }
    for found in &read {
        let plain = found
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
        let keyword = ["class", "struct", "record", "enum"].contains(&found.as_str());
        assert!(
            plain && !keyword,
            "{extension}: misnamed {found} in {read:?}"
        );
    }
}

#[test]
fn every_declaration_form_is_read_by_its_name() {
    assert_names(
        "ts",
        "export interface Order {\n  id: string\n}\nexport type Status = 'open' | 'paid'\nexport type Box<T> = { value: T }\ninterface Page<T> {\n  items: T[]\n}\n",
        &["Order", "Status", "Box", "Page"],
    );
    assert_names(
        "rs",
        "pub struct Order {\n    pub id: String,\n}\npub struct Id(String);\npub enum Status {\n    Open,\n}\npub type Ids = Vec<Id>;\n",
        &["Order", "Id", "Status", "Ids"],
    );
    assert_names(
        "cs",
        "public record Order(string Id);\npublic record struct Point(int X);\npublic readonly record struct Size(int Width);\npublic record class Box(int Width);\npublic sealed class Invoice { public string Id { get; init; } = \"\"; }\npublic enum Status { Open }\n",
        &["Order", "Point", "Size", "Box", "Invoice", "Status"],
    );
    assert_names(
        "java",
        "record Item(String name) {}\npublic record Page<T>(java.util.List<T> items) {}\nenum Status { OPEN }\npublic final class Order {\n    public String id;\n}\n",
        &["Item", "Page", "Status", "Order"],
    );
    assert_names(
        "kt",
        "data class Order(val id: String)\nenum class Status { OPEN }\ntypealias Ids = List<String>\ndata class Page<T>(val items: List<T>)\n",
        &["Order", "Status", "Ids", "Page"],
    );
    assert_names(
        "swift",
        "struct Order {\n    let id: String\n}\nenum Status: String {\n    case open\n}\ntypealias Ids = [String]\n",
        &["Order", "Status", "Ids"],
    );
    assert_names(
        "dart",
        "class Order {\n  final String id;\n  const Order(this.id);\n}\nenum Status { open }\ntypedef Ids = List<String>;\n",
        &["Order", "Status", "Ids"],
    );
    assert_names(
        "php",
        "<?php\nclass Order {\n    public string $id;\n}\nfinal readonly class Line {\n    public string $sku;\n}\nenum Status: string {\n    case Open = 'open';\n}\n",
        &["Order", "Line", "Status"],
    );
    assert_names(
        "py",
        "@dataclass(frozen=True)\nclass Order:\n    id: Annotated[str, 'id']\n\nclass Status(Enum):\n    OPEN = 'open'\n\nclass Row(TypedDict, total=False):\n    name: str\n",
        &["Order", "Status", "Row"],
    );
    assert_names(
        "rb",
        "Message = Data.define(:name)\nclass Invoice\n  attr_reader :id\n\n  def initialize(id:)\n    @id = id\n  end\nend\nclass Line < Data.define(:sku)\nend\n",
        &["Message", "Invoice", "Line"],
    );
    assert_names(
        "go",
        "type Page[T any] struct {\n\tItems []T `json:\"items\"`\n}\ntype Status string\ntype Ids = []string\n",
        &["Page", "Status", "Ids"],
    );
    assert_names(
        "js",
        "/**\n * @typedef {Object} Order\n * @property {string} id\n */\n/** @typedef {'open' | 'paid'} Status */\n/**\n * @typedef Plain\n * @property {number} count\n */\n",
        &["Order", "Status", "Plain"],
    );
}

#[test]
fn a_record_class_and_a_generic_go_struct_keep_their_fields() {
    let csharp = table("cs", "public record class Box(int Width);\n");
    assert!(csharp.is_record("Box"), "{csharp:?}");
    let go = table(
        "go",
        "type Page[T any] struct {\n\tTotal int `json:\"total\"`\n}\n",
    );
    assert!(go.is_record("Page"), "{go:?}");
    let described = go.event_models("Page").expect("page model");
    assert_eq!(described["Page"]["fields"]["total"], "!integer");
}

#[test]
fn type_aliases_resolve_to_their_shape() {
    let kotlin = table(
        "kt",
        "typealias Ids = List<String>\ndata class Order(val ids: Ids)\n",
    );
    let described = kotlin.event_models("Order").expect("order model");
    assert_eq!(described["Order"]["fields"]["ids"], "![]string");
    let rust = table(
        "rs",
        "pub type Count = i64;\npub struct Total {\n    pub count: Count,\n}\n",
    );
    let described = rust.event_models("Total").expect("total model");
    assert_eq!(described["Total"]["fields"]["count"], "!integer");
}
