use super::FileNameCheck;
use std::path::Path;

fn codes(path: &str, source: &str) -> Vec<u16> {
    crate::annotation_layout::validate(Path::new(path), source)
        .err()
        .map(|failure| {
            failure
                .diagnostics()
                .iter()
                .map(|found| found.code.number())
                .collect()
        })
        .unwrap_or_default()
}

fn help(path: &str, source: &str) -> String {
    let failure =
        crate::annotation_layout::validate(Path::new(path), source).expect_err("refused file");
    let found = &failure.diagnostics()[0];
    format!(
        "{} {}",
        found.message,
        found.help.clone().unwrap_or_default()
    )
}

const MATCHING: [(&str, &str); 14] = [
    (
        "client/tests/ShellServiceTest.js",
        "class ShellServiceTest {}\n",
    ),
    (
        "server/models/Order.ts",
        "export interface Order {\n  id: string\n}\n",
    ),
    (
        "server/models/Order.py",
        "from dataclasses import dataclass\n\n@dataclass\nclass Order:\n    id: str\n",
    ),
    ("server/models/Order.rb", "Order = Data.define(:id)\n"),
    (
        "server/models/Order.go",
        "package main\n\ntype Order struct {\n\tID string\n}\n",
    ),
    (
        "server/models/Order.rs",
        "pub struct Order {\n    pub id: String,\n}\n",
    ),
    (
        "server/models/Order.java",
        "public record Order(String id) {}\n",
    ),
    (
        "server/models/Order.kt",
        "data class Order(val id: String)\n",
    ),
    (
        "server/models/Order.cs",
        "public record Order(string Id);\n",
    ),
    (
        "server/models/Order.dart",
        "class Order {\n  final String id;\n  Order(this.id);\n}\n",
    ),
    (
        "server/models/Order.php",
        "<?php\nclass Order {\n    public string $id;\n}\n",
    ),
    (
        "client/models/Order.swift",
        "struct Order {\n    let id: String\n}\n",
    ),
    (
        "server/helpers/forecast.py",
        "def forecast():\n    return 1\n",
    ),
    (
        "client/tests/TotalsTest.ts",
        "interface Row { total: number }\nexport class TotalsTest {}\n",
    ),
];

const MISMATCHED: [(&str, &str); 12] = [
    ("client/tests/ShellServiceTest.js", "class ShellChecks {}\n"),
    (
        "server/models/Order.ts",
        "export interface Invoice {\n  id: string\n}\n",
    ),
    (
        "server/models/Order.py",
        "from dataclasses import dataclass\n\n@dataclass\nclass Invoice:\n    id: str\n",
    ),
    ("server/models/Order.rb", "Invoice = Data.define(:id)\n"),
    (
        "server/models/Order.go",
        "package main\n\ntype Invoice struct {\n\tID string\n}\n",
    ),
    (
        "server/models/Order.rs",
        "pub struct Invoice {\n    pub id: String,\n}\n",
    ),
    (
        "server/models/Order.java",
        "public record Invoice(String id) {}\n",
    ),
    (
        "server/models/Order.kt",
        "data class Invoice(val id: String)\n",
    ),
    (
        "server/models/Order.cs",
        "public record Invoice(string Id);\n",
    ),
    (
        "server/models/Order.dart",
        "class Invoice {\n  final String id;\n  Invoice(this.id);\n}\n",
    ),
    (
        "server/models/Order.php",
        "<?php\nclass Invoice {\n    public string $id;\n}\n",
    ),
    (
        "client/models/Order.swift",
        "struct Invoice {\n    let id: String\n}\n",
    ),
];

#[test]
fn a_class_file_named_after_its_class_passes() {
    for (path, source) in MATCHING {
        assert_eq!(codes(path, source), Vec::<u16>::new(), "{path}");
    }
}

#[test]
fn a_class_file_with_another_name_fails_ty1318() {
    for (path, source) in MISMATCHED {
        assert_eq!(codes(path, source), vec![FileNameCheck::CODE], "{path}");
    }
}

#[test]
fn the_message_names_the_rename() {
    let text = help(
        "server/models/Order.ts",
        "export interface Invoice {\n  id: string\n}\n",
    );
    assert!(
        text.contains("'Order' does not match the class 'Invoice'"),
        "{text}"
    );
    assert!(text.contains("Rename the file to 'Invoice.ts'"), "{text}");
    assert!(text.contains("ty migrate fix --write"), "{text}");
    let go = help(
        "server/models/Order.go",
        "package main\n\ntype Invoice struct{}\n",
    );
    assert!(go.contains("the type 'Invoice'"), "{go}");
}

#[test]
fn a_models_file_declares_the_model_it_is_named_after() {
    let alias = "export type Status = 'open' | 'paid'\n";
    assert_eq!(codes("client/models/Status.ts", alias), Vec::<u16>::new());
    assert_eq!(
        codes("client/models/Order.ts", alias),
        vec![FileNameCheck::CODE]
    );
    let text = help("client/models/Order.ts", alias);
    assert!(text.contains("the class 'Status'"), "{text}");
    let rust_alias = "pub type Ids = Vec<String>;\n";
    assert_eq!(codes("server/models/Ids.rs", rust_alias), Vec::<u16>::new());
    assert_eq!(
        codes("server/models/Order.rs", rust_alias),
        vec![FileNameCheck::CODE]
    );
    let both = "export interface Order {\n  id: string\n}\nexport type Status = 'open' | 'paid'\n";
    let text = help("client/models/Order.ts", both);
    assert!(text.contains("declares 2 models (Order, Status)"), "{text}");
}

#[test]
fn a_record_struct_and_a_generic_model_are_named_after_their_type() {
    let record = "public readonly record struct Point(int X, int Y);\n";
    assert_eq!(codes("server/models/Point.cs", record), Vec::<u16>::new());
    assert_eq!(
        codes("server/models/Size.cs", record),
        vec![FileNameCheck::CODE]
    );
    let page = "package main\n\ntype Page[T any] struct {\n\tItems []T\n}\n";
    assert_eq!(codes("server/models/Page.go", page), Vec::<u16>::new());
}

#[test]
fn a_test_setup_and_a_utility_follow_the_rule() {
    let setup = "class SiteSetup {}\n";
    assert_eq!(
        codes("client/tests/SiteTestSetup.js", setup),
        vec![FileNameCheck::CODE]
    );
    assert_eq!(codes("client/tests/SiteSetup.js", setup), Vec::<u16>::new());
}

#[test]
fn views_shared_code_workers_and_middleware_keep_their_names() {
    let free = [
        "client/pages/tac.ts",
        "client/components/card/tac.ts",
        "client/shared/scripts/imports.ts",
        "server/shared/orders.ts",
        "server/workers/cleanup.ts",
        "middleware.ts",
        "client/Order.ts",
    ];
    for path in free {
        let checked = FileNameCheck::check(Path::new(path), "export class Handler {}\n");
        assert!(checked.is_ok(), "{path}: {checked:?}");
    }
}

#[test]
fn the_wanted_name_is_the_only_declared_class() {
    let wanted = |path: &str, source: &str| FileNameCheck::wanted_name(Path::new(path), source);
    let invoice = "export interface Invoice {\n  id: string\n}\n";
    assert_eq!(
        wanted("server/models/Order.ts", invoice).as_deref(),
        Some("Invoice")
    );
    assert_eq!(wanted("server/models/Invoice.ts", invoice), None);
    assert_eq!(wanted("server/shared/order.ts", invoice), None);
    let two = "class First {}\nclass Second {}\n";
    assert_eq!(wanted("server/services/Order.ts", two), None);
}
