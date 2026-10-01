use super::ModelBlocks;
use std::path::Path;

fn names(file: &str, source: &str) -> Vec<String> {
    ModelBlocks::names(Path::new(file), source).expect("model names")
}

fn texts(file: &str, source: &str) -> Vec<String> {
    ModelBlocks::read(Path::new(file), source)
        .expect("model blocks")
        .iter()
        .map(|block| block.text(source).to_owned())
        .collect()
}

#[test]
fn typescript_interfaces_and_type_aliases_are_named() {
    let source = "import type { Base } from './Base.ts'\n\nexport interface Order extends Base {\n  id: string\n}\ninterface Page<T> {\n  items: T[]\n}\nexport type Status = 'open' | 'closed'\nexport type Box<T> = { value: T }\ntype Joined = Order &\n  { note: string }\ndeclare interface Declared {}\nexport default interface Main {}\nexport type { Order as Other } from './Other.ts'\n";
    assert_eq!(
        names("server/models/Order.ts", source),
        [
            "Order", "Page", "Status", "Box", "Joined", "Declared", "Main"
        ]
    );
}

#[test]
fn rust_structs_enums_and_type_aliases_are_named() {
    let source = "use crate::Json;\n\n#[derive(Clone)]\npub struct Order {\n    pub id: String,\n}\npub struct Id(String);\npub(crate) struct Page<T> {\n    pub items: Vec<T>,\n}\npub enum Status {\n    Open,\n}\npub type Ids = Vec<Id>;\nstruct Unit;\n";
    assert_eq!(
        names("server/models/Order.rs", source),
        ["Order", "Id", "Page", "Status", "Ids", "Unit"]
    );
}

#[test]
fn csharp_records_classes_structs_and_enums_are_named() {
    let source = "using System.Collections.Generic;\n\npublic record Order(string Id);\npublic record struct Point(int X, int Y);\npublic readonly record struct Size(int Width);\npublic record class Box(int Width);\n[Serializable]\npublic sealed class Invoice\n{\n    public string Id { get; init; } = \"\";\n    public class Nested { }\n}\npublic struct Pair { public int Left; }\npublic enum Status { Open, Closed }\npublic record Page<T>(List<T> Items);\npublic partial class Split { }\n";
    assert_eq!(
        names("server/models/Order.cs", source),
        [
            "Order", "Point", "Size", "Box", "Invoice", "Pair", "Status", "Page", "Split"
        ]
    );
}

#[test]
fn java_records_classes_and_enums_are_named() {
    let source = "import java.util.List;\n\nrecord Item(String name, long level) {}\npublic record Page<T>(List<T> items) {}\nenum Status { OPEN, CLOSED }\npublic final class Order {\n    public String id;\n    public static class Line {}\n}\n@Deprecated\nclass Old {}\n";
    assert_eq!(
        names("server/models/Item.java", source),
        ["Item", "Page", "Status", "Order", "Old"]
    );
}

#[test]
fn kotlin_classes_enums_and_type_aliases_are_named() {
    let source = "import kotlinx.serialization.Serializable\n\n@Serializable\ndata class Order(val id: String)\ndata class Page<T>(\n    val items: List<T>,\n)\nenum class Status { OPEN, CLOSED }\nclass Box(val width: Int) {\n    val area: Int = width\n}\ntypealias Ids = List<String>\nsealed class Shape\nvalue class Token(val text: String)\n";
    assert_eq!(
        names("client/models/Order.kt", source),
        ["Order", "Page", "Status", "Box", "Ids", "Shape", "Token"]
    );
}

#[test]
fn swift_structs_enums_and_type_aliases_are_named() {
    let source = "import Foundation\n\nstruct Order: Codable {\n    let id: String\n}\npublic struct Page<T> {\n    let items: [T]\n}\nenum Status: String {\n    case open\n}\ntypealias Ids = [String]\nfinal class Box {\n    let width: Int = 0\n}\n";
    assert_eq!(
        names("client/models/Order.swift", source),
        ["Order", "Page", "Status", "Ids", "Box"]
    );
}

#[test]
fn dart_classes_enums_and_typedefs_are_named() {
    let source = "import 'Base.dart';\n\nclass Order {\n  final String id;\n  const Order(this.id);\n}\nclass Page<T> {\n  final List<T> items;\n  const Page(this.items);\n}\nenum Status { open, closed }\ntypedef Ids = List<String>;\nfinal class Sealed {}\nabstract interface class Shape {}\nmixin class Mixed {}\n";
    assert_eq!(
        names("client/models/Order.dart", source),
        ["Order", "Page", "Status", "Ids", "Sealed", "Shape", "Mixed"]
    );
}

#[test]
fn php_classes_and_enums_are_named() {
    let source = "<?php\nuse Models\\Base;\n\nclass Message {\n    public function __construct(public string $name) {}\n}\nfinal readonly class Order {\n    public string $id;\n}\nenum Status: string {\n    case Open = 'open';\n}\n";
    assert_eq!(
        names("server/models/Message.php", source),
        ["Message", "Order", "Status"]
    );
}

#[test]
fn python_dataclasses_and_typed_classes_are_named() {
    let source = "from dataclasses import dataclass\nfrom enum import Enum\nfrom typing import Annotated, Generic, TypedDict, TypeVar\n\n\n@dataclass(frozen=True)\nclass Order:\n    id: Annotated[str, 'id']\n    total: int = 0\n\n\nclass Status(Enum):\n    OPEN = 'open'\n\n\nclass Row(TypedDict, total=False):\n    name: str\n\n\n@dataclass\nclass Page(Generic[T]):\n    items: list[T]\n";
    assert_eq!(
        names("server/models/Order.py", source),
        ["Order", "Status", "Row", "Page"]
    );
}

#[test]
fn ruby_records_and_classes_are_named() {
    let source = "require 'json'\n\n# @!attribute [r] name\n#   @return [String]\nMessage = Data.define(:name)\nOrder = Struct.new(\n  :id,\n  keyword_init: true\n)\nclass Invoice\n  attr_reader :id\n\n  def initialize(id:)\n    @id = id\n  end\nend\nclass Line < Data.define(:sku)\nend\nclass Models::Receipt < Struct.new(:id)\nend\n";
    assert_eq!(
        names("server/models/Message.rb", source),
        ["Message", "Order", "Invoice", "Line", "Receipt"]
    );
}

#[test]
fn go_structs_generics_and_aliases_are_named() {
    assert_eq!(
        names(
            "server/models/Page.go",
            "package main\n\ntype Page[T any] struct {\n\tItems []T `json:\"items\"`\n}\n"
        ),
        ["Page"]
    );
    assert_eq!(
        names(
            "server/models/Pair.go",
            "type Pair[K comparable, V any] struct {\n\tKey K\n}\n"
        ),
        ["Pair"]
    );
    assert_eq!(
        names("server/models/Status.go", "type Status string\n"),
        ["Status"]
    );
    assert_eq!(
        names("server/models/Ids.go", "type Ids = []string\n"),
        ["Ids"]
    );
}

#[test]
fn javascript_typedefs_are_named() {
    let source = "/**\n * @typedef {Object} Order\n * @property {string} id\n */\n/** @typedef {{ id: string, lines: Array<{ sku: string }> }} Invoice */\n/**\n * @typedef Plain\n * @property {number} count\n */\n/** A note without a type. */\n";
    assert_eq!(
        names("client/pages/tac.js", source),
        ["Order", "Invoice", "Plain"]
    );
}

#[test]
fn a_block_holds_its_comments_attributes_and_whole_lines() {
    let source = "use serde::Serialize;\n\n/// The order.\n#[derive(Serialize)]\npub struct Order {\n    pub id: String,\n} // done\n\n// The invoice.\npub struct Invoice;\n";
    assert_eq!(
        texts("server/models/Order.rs", source),
        [
            "/// The order.\n#[derive(Serialize)]\npub struct Order {\n    pub id: String,\n} // done\n",
            "// The invoice.\npub struct Invoice;\n"
        ]
    );
}

#[test]
fn ruby_yard_comments_and_python_decorators_stay_with_their_model() {
    let ruby = "# @!attribute [r] id\n#   @return [String]\nOrder = Data.define(:id)\n\n# @!attribute [r] sku\n#   @return [String]\nLine = Data.define(:sku)\n";
    assert_eq!(
        texts("server/models/Order.rb", ruby),
        [
            "# @!attribute [r] id\n#   @return [String]\nOrder = Data.define(:id)\n",
            "# @!attribute [r] sku\n#   @return [String]\nLine = Data.define(:sku)\n"
        ]
    );
    let python = "from dataclasses import dataclass\n\n\n@dataclass\nclass Order:\n    id: str\n\n\n@dataclass\nclass Line:\n    sku: str\n";
    assert_eq!(
        texts("server/models/Order.py", python),
        [
            "@dataclass\nclass Order:\n    id: str\n",
            "@dataclass\nclass Line:\n    sku: str\n"
        ]
    );
}

#[test]
fn kotlin_annotations_and_adjacent_typedefs_stay_apart() {
    let kotlin = "@Serializable\ndata class Order(val id: String)\n@Serializable\ndata class Line(val sku: String)\n";
    assert_eq!(
        texts("client/models/Order.kt", kotlin),
        [
            "@Serializable\ndata class Order(val id: String)\n",
            "@Serializable\ndata class Line(val sku: String)\n"
        ]
    );
    let script = "/** @typedef {Object} Order */\n/** @typedef {Object} Line */\n";
    assert_eq!(
        texts("client/pages/tac.js", script),
        [
            "/** @typedef {Object} Order */\n",
            "/** @typedef {Object} Line */\n"
        ]
    );
}

#[test]
fn imports_and_comments_alone_declare_no_model() {
    assert!(
        names(
            "server/models/Empty.ts",
            "import type { A } from './A.ts'\n// type Hidden = string\n"
        )
        .is_empty()
    );
    assert!(
        names(
            "server/models/Empty.java",
            "import java.util.List;\n/* class Hidden {} */\n"
        )
        .is_empty()
    );
    assert!(names("server/models/Empty.py", "import os\n# class Hidden:\n").is_empty());
}

#[test]
fn text_in_strings_does_not_join_or_hide_models() {
    let typescript = "export type Status = 'open' | 'closed'\nexport type Mode = \"a | b\"\nexport interface Note {\n  text: string\n}\n";
    assert_eq!(
        names("server/models/Status.ts", typescript),
        ["Status", "Mode", "Note"]
    );
    let kotlin = "data class Label(val text: String = \"{ class Hidden\")\nenum class Kind { A }\n";
    assert_eq!(names("client/models/Label.kt", kotlin), ["Label", "Kind"]);
}

#[test]
fn javascript_schema_classes_and_jsdoc_types_keep_their_owned_blocks() {
    let source = "/** @typedef {object} Receipt */\n\n@Checked\nclass OrderInput {\n  @Required\n  name = 'class Fake {}';\n}\n\nconst decoy = \"class Other {}\";\n";
    assert_eq!(
        names("server/models/OrderInput.js", source),
        ["Receipt", "OrderInput"]
    );
    let blocks = texts("server/models/OrderInput.js", source);
    assert!(blocks[0].contains("@typedef") && !blocks[0].contains("class OrderInput"));
    assert!(blocks[1].starts_with("@Checked") && blocks[1].contains("class OrderInput"));
    assert!(!blocks[1].contains("const decoy"));
}

#[test]
fn ordinary_javascript_service_classes_are_not_inline_models() {
    let path = Path::new("client/services/InputService.js");
    let source =
        "@Service\nexport default class InputService { accept(value) { return value; } }\n";
    assert!(
        ModelBlocks::read(path, source)
            .expect("ordinary source")
            .is_empty()
    );
    crate::annotation_layout::InlineModelCheck::refuse_inline_models(path, source)
        .expect("service stays a service");
}
