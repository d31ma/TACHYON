use super::OneModelCheck;
use crate::annotation_layout::FileNameCheck;
use std::path::Path;

fn refusal(path: &str, source: &str) -> Option<(u16, String)> {
    let failure = crate::annotation_layout::validate(Path::new(path), source).err()?;
    let found = &failure.diagnostics()[0];
    let text = format!(
        "{} {}",
        found.message,
        found.help.clone().unwrap_or_default()
    );
    Some((found.code.number(), text))
}

const TWO_MODELS: [(&str, &str); 11] = [
    (
        "server/models/Order.ts",
        "export interface Order {\n  id: string\n}\nexport type Status = 'open' | 'paid'\n",
    ),
    (
        "server/models/Order.rs",
        "pub struct Order {\n    pub id: String,\n}\npub enum Status {\n    Open,\n}\n",
    ),
    (
        "server/models/Order.cs",
        "public record Order(string Id);\npublic record struct Status(string Name);\n",
    ),
    (
        "server/models/Order.java",
        "record Order(String id) {}\nenum Status { OPEN }\n",
    ),
    (
        "server/models/Order.kt",
        "data class Order(val id: String)\nenum class Status { OPEN }\n",
    ),
    (
        "client/models/Order.swift",
        "struct Order {\n    let id: String\n}\ntypealias Status = String\n",
    ),
    (
        "client/models/Order.dart",
        "class Order {\n  final String id;\n  const Order(this.id);\n}\ntypedef Status = String;\n",
    ),
    (
        "server/models/Order.php",
        "<?php\nclass Order {\n    public string $id;\n}\nenum Status: string {\n    case Open = 'open';\n}\n",
    ),
    (
        "server/models/Order.py",
        "from dataclasses import dataclass\nfrom enum import Enum\n\n\n@dataclass\nclass Order:\n    id: str\n\n\nclass Status(Enum):\n    OPEN = 'open'\n",
    ),
    (
        "server/models/Order.rb",
        "Order = Data.define(:id)\nStatus = Struct.new(:name)\n",
    ),
    (
        "client/models/Order.rb",
        "class Order\n  attr_reader :id\n\n  def initialize(id:)\n    @id = id\n  end\nend\nStatus = Data.define(:name)\n",
    ),
];

#[test]
fn a_models_file_with_two_models_fails_ty1318() {
    for (path, source) in TWO_MODELS {
        let (code, text) = refusal(path, source).unwrap_or_default();
        assert_eq!(code, FileNameCheck::CODE, "{path}: {text}");
        assert!(
            text.contains("declares 2 models (Order, Status)"),
            "{path}: {text}"
        );
        assert!(
            text.contains("Move Status into its own file 'Status."),
            "{path}: {text}"
        );
        assert!(text.contains("ty migrate fix --write"), "{path}: {text}");
    }
}

#[test]
fn a_models_file_with_one_model_and_its_imports_passes() {
    let one = [
        (
            "server/models/OrderList.ts",
            "import type { Order } from './Order.ts'\n\nexport interface OrderList {\n  orders: Order[]\n}\n",
        ),
        (
            "server/models/Invoice.cs",
            "using System.Collections.Generic;\n\npublic sealed class Invoice\n{\n    public List<string> Lines { get; init; } = new();\n}\n",
        ),
        (
            "server/models/Report.rs",
            "use crate::Json;\n\n#[derive(Clone)]\npub struct Report {\n    pub summary: Json,\n}\n",
        ),
    ];
    for (path, source) in one {
        assert_eq!(refusal(path, source), None, "{path}");
    }
}

#[test]
fn files_outside_models_are_not_counted() {
    let path = Path::new("server/services/OrderService.ts");
    assert!(!OneModelCheck::applies(path));
    assert!(OneModelCheck::applies(Path::new("client/models/Order.js")));
    assert!(OneModelCheck::applies(Path::new("client/models/Order.ts")));
    let typedefs = "/** @typedef {Object} Order */\n/** @typedef {Object} Line */\n";
    assert!(OneModelCheck::check(Path::new("client/pages/tac.js"), typedefs).is_ok());
    assert!(OneModelCheck::check(Path::new("client/models/Order.js"), typedefs).is_err());
}

#[test]
fn the_error_points_at_the_second_model() {
    let source =
        "export interface Order {\n  id: string\n}\n\nexport interface Line {\n  sku: string\n}\n";
    let failure =
        OneModelCheck::check(Path::new("client/models/Order.ts"), source).expect_err("two models");
    let span = &failure.diagnostics()[0].spans[0];
    assert_eq!(
        usize::try_from(span.start).unwrap_or(0),
        source.find("export interface Line").unwrap_or(0)
    );
}
