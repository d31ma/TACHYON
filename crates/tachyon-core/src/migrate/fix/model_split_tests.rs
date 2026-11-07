use super::MigrationFix;
use super::visibility_off::VisibilityOff;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn project(files: &[(&str, &str)]) -> TempDir {
    let project = tempfile::tempdir().expect("project");
    for (relative, contents) in files {
        let path = project.path().join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("source");
    }
    VisibilityOff::write(project.path());
    project
}

fn fix(root: &Path) -> String {
    let planned = MigrationFix::plan(root).expect("plan");
    let text = planned.to_text(false);
    planned.write(root).expect("write");
    text
}

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).expect(relative)
}

fn passes(root: &Path, relative: &str) {
    let source = read(root, relative);
    let checked = crate::annotation_layout::validate(Path::new(relative), &source);
    assert!(checked.is_ok(), "{relative}: {checked:?}\n{source}");
}

#[test]
fn a_typescript_models_file_is_split_and_its_importers_follow() {
    let project = project(&[
        (
            "server/models/Order.ts",
            "import type { Money } from './Money.ts'\n\nexport interface Order {\n  id: string\n  invoice: Invoice\n}\n\nexport interface Invoice {\n  order: Order\n  total: Money\n}\n\nexport type Status = 'open' | 'paid'\n",
        ),
        ("server/models/Money.ts", "export type Money = number\n"),
        (
            "server/services/OrderService.ts",
            "import type { Order, Invoice } from '/models/Order.ts'\nimport type { Money } from '/models/Money.ts'\n\n/** @param {import('/models/Order.ts').Status} status */\nexport default class OrderService {}\n",
        ),
    ]);
    let root = project.path();
    let text = fix(root);
    assert!(text.contains("server/models/Invoice.ts"), "{text}");
    assert_eq!(
        read(root, "server/models/Order.ts"),
        "import type { Money } from './Money.ts'\nimport type { Invoice } from './Invoice.ts'\n\nexport interface Order {\n  id: string\n  invoice: Invoice\n}\n"
    );
    assert_eq!(
        read(root, "server/models/Invoice.ts"),
        "import type { Money } from './Money.ts'\nimport type { Order } from './Order.ts'\n\nexport interface Invoice {\n  order: Order\n  total: Money\n}\n"
    );
    assert_eq!(
        read(root, "server/models/Status.ts"),
        "import type { Money } from './Money.ts'\n\nexport type Status = 'open' | 'paid'\n"
    );
    assert_eq!(
        read(root, "server/services/OrderService.ts"),
        "import type { Order } from '/models/Order.ts'\nimport type { Invoice } from '/models/Invoice.ts'\nimport type { Money } from '/models/Money.ts'\n\n/** @param {import('/models/Status.ts').Status} status */\nexport default class OrderService {}\n"
    );
    for model in ["Order", "Invoice", "Status"] {
        passes(root, &format!("server/models/{model}.ts"));
    }
}

#[test]
fn python_ruby_and_rust_models_keep_their_imports_comments_and_decorators() {
    let project = project(&[
        (
            "server/models/Order.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Line:\n    sku: str\n\n\n@dataclass\nclass Order:\n    lines: list[Line]\n",
        ),
        (
            "server/models/Order.rb",
            "# @!attribute [r] id\n#   @return [String]\nOrder = Data.define(:id)\n\n# @!attribute [r] sku\n#   @return [String]\nLine = Data.define(:sku)\n",
        ),
        (
            "server/models/Report.rs",
            "use crate::Json;\n\n#[derive(Clone)]\npub struct Report {\n    pub summary: Json,\n    pub rows: Vec<Row>,\n}\n\npub struct Row {\n    pub cells: Json,\n}\n",
        ),
    ]);
    let root = project.path();
    fix(root);
    assert_eq!(
        read(root, "server/models/Order.py"),
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Order:\n    lines: list[Line]\n"
    );
    assert_eq!(
        read(root, "server/models/Line.py"),
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Line:\n    sku: str\n"
    );
    assert_eq!(
        read(root, "server/models/Order.rb"),
        "# @!attribute [r] id\n#   @return [String]\nOrder = Data.define(:id)\n"
    );
    assert_eq!(
        read(root, "server/models/Line.rb"),
        "# @!attribute [r] sku\n#   @return [String]\nLine = Data.define(:sku)\n"
    );
    assert_eq!(
        read(root, "server/models/Report.rs"),
        "use crate::Json;\nuse crate::models::Row;\n\n#[derive(Clone)]\npub struct Report {\n    pub summary: Json,\n    pub rows: Vec<Row>,\n}\n"
    );
    assert_eq!(
        read(root, "server/models/Row.rs"),
        "use crate::Json;\n\npub struct Row {\n    pub cells: Json,\n}\n"
    );
    for model in [
        "Order.py",
        "Line.py",
        "Order.rb",
        "Line.rb",
        "Report.rs",
        "Row.rs",
    ] {
        passes(root, &format!("server/models/{model}"));
    }
}

#[test]
fn dart_importers_gain_an_import_for_each_moved_model() {
    let project = project(&[
        (
            "client/models/Order.dart",
            "class Order {\n  final String id;\n  const Order(this.id);\n}\n\nenum Status { open, paid }\n",
        ),
        (
            "client/services/OrderService.dart",
            "import 'models/Order.dart';\n\nclass OrderService {\n  Status status(Order order) => Status.open;\n}\n",
        ),
    ]);
    let root = project.path();
    fix(root);
    assert_eq!(
        read(root, "client/models/Order.dart"),
        "class Order {\n  final String id;\n  const Order(this.id);\n}\n"
    );
    assert_eq!(
        read(root, "client/models/Status.dart"),
        "enum Status { open, paid }\n"
    );
    assert_eq!(
        read(root, "client/services/OrderService.dart"),
        "import 'models/Order.dart';\nimport 'models/Status.dart';\n\nclass OrderService {\n  Status status(Order order) => Status.open;\n}\n"
    );
}

#[test]
fn a_models_file_is_left_alone_when_a_target_file_exists() {
    let two = "public record Order(string Id);\npublic record Line(string Sku);\n";
    let project = project(&[
        ("server/models/Order.cs", two),
        ("server/models/Line.cs", "public record Line(string Sku);\n"),
    ]);
    let root = project.path();
    let text = fix(root);
    assert!(
        text.contains("'server/models/Line.cs' already exists"),
        "{text}"
    );
    assert_eq!(read(root, "server/models/Order.cs"), two);
}

#[test]
fn a_file_named_after_none_of_its_models_keeps_the_first() {
    let project = project(&[(
        "client/models/Shapes.kt",
        "data class Circle(val radius: Double)\n\ndata class Square(val side: Double)\n",
    )]);
    let root = project.path();
    fix(root);
    assert_eq!(
        read(root, "client/models/Shapes.kt"),
        "data class Circle(val radius: Double)\n"
    );
    assert_eq!(
        read(root, "client/models/Square.kt"),
        "data class Square(val side: Double)\n"
    );
    fix(root);
    assert_eq!(
        read(root, "client/models/Circle.kt"),
        "data class Circle(val radius: Double)\n"
    );
}
