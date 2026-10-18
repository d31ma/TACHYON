use super::CppYon;
use super::role_tests::Roles;
use std::path::Path;

struct Checks;

impl Checks {
    const CONTROLLER_PATH: &'static str = "server/controllers/OrdersController.hpp";
    const CONTROLLER: &'static str = r#"#pragma once
#include "../services/OrderService.hpp"

class [[RequestRoute("/orders")]] [[Public]] OrdersController {
    [[Needs]] OrderService& orders;
    [[Needs]] OrderConverter& converter;

public:
    [[PostRoute]]
    OrderSummary postOrders([[FromBody]] const OrderInput& order) { return orders.accept(order); }

    [[PostRoute("/direct")]]
    OrderInput postDirect([[FromBody]] [[Check]] const OrderInput& order) { return order; }

    [[PostRoute("/product")]]
    tachyon::json::JsonNode postProduct([[FromBody]] const tachyon::json::JsonNode& value) {
        return converter.toProduct(value);
    }
};
"#;
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "server/models/OrderInput.hpp",
            r#"#pragma once
#include <optional>
#include <string>
#include <vector>

struct [[Checked]] OrderInput {
    [[NotEmpty]] [[Length({"min":2,"max":8})]] std::string name;
    [[Required]] [[Min(1)]] [[Max(9)]] std::optional<int> level;
    [[Length({"max":2})]] std::vector<std::string> tags;
};
"#,
        ),
        (
            "server/models/OrderSummary.hpp",
            "#pragma once\n#include <optional>\n#include <string>\n\nstruct OrderSummary {\n    std::string label;\n    std::optional<int> level;\n    std::string kind;\n};\n",
        ),
        (
            "server/converters/OrderConverter.hpp",
            r#"#pragma once

class [[Converter]] OrderConverter {
public:
    [[Convert({"fields":[{"source":"name","target":"label"},{"target":"kind","constant":"order"}]})]]
    OrderSummary toSummary(const OrderInput& order) const;

    [[Convert({"fields":[{"source":"detail.id","target":"productId"},{"target":"kind","constant":"product"}]})]]
    tachyon::json::JsonNode toProduct(const tachyon::json::JsonNode& value) const;
};
"#,
        ),
        (
            "server/services/OrderService.hpp",
            r"#pragma once

class [[Service]] OrderService {
    [[Needs]] OrderConverter& converter;

public:
    OrderSummary accept([[Check]] const OrderInput& order) { return converter.toSummary(order); }
};
",
        ),
    ];

    fn project(changes: &[(&str, &str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in Self::FILES {
            let edited = changes
                .iter()
                .filter(|(file, _, _)| *file == relative)
                .fold(source.to_owned(), |text, (_, from, to)| {
                    assert!(text.contains(from), "{relative} has {from}");
                    text.replace(from, to)
                });
            Roles::write(root.path(), relative, &edited);
        }
        root
    }

    fn program(controller: &str) -> String {
        let root = Self::project(&[]);
        CppYon::lower(Path::new(Self::CONTROLLER_PATH), controller, root.path())
            .expect("lowered")
            .source
    }

    fn refusal(changes: &[(&str, &str, &str)], controller: &str) -> String {
        let root = Self::project(changes);
        CppYon::lower(Path::new(Self::CONTROLLER_PATH), controller, root.path())
            .err()
            .expect("the project is refused")
            .to_string()
    }

    fn answer(program: &str, route: &str, body: &serde_json::Value) -> serde_json::Value {
        let frame = serde_json::json!({"method": "POST", "route": route, "arguments": [body]});
        let (answer, errors) = Roles::run(program, &frame.to_string());
        assert!(errors.is_empty(), "{errors}");
        answer
    }

    fn body(answer: &serde_json::Value) -> serde_json::Value {
        serde_json::from_str(answer["body"].as_str().expect("body")).expect("body JSON")
    }
}

#[test]
fn checked_models_converters_and_check_guards_are_composed() {
    let program = Checks::program(Checks::CONTROLLER);
    assert!(program.contains("class TacChecks<::OrderInput>"));
    assert!(program.contains("::tachyon::TacCheckReport::check<OrderInput>(order);"));
    assert!(program.contains("inline OrderSummary OrderConverter::toSummary("));
    assert!(program.contains("inline tachyon::json::JsonNode OrderConverter::toProduct("));
    assert!(!program.contains("[[Check]]"));
    assert!(!program.contains("[[Converter]]"));
}

#[test]
fn a_checked_argument_reaches_the_method_and_a_broken_rule_is_reported() {
    if !Roles::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let program = Checks::program(Checks::CONTROLLER);
    let accepted = Checks::answer(
        &program,
        "/orders",
        &serde_json::json!({"name": "ab", "level": 3, "sku": "A-1"}),
    );
    assert_eq!(accepted["status"], 200, "{accepted}");
    assert_eq!(
        Checks::body(&accepted),
        serde_json::json!({"label": "ab", "level": 3, "kind": "order"})
    );

    let rejected = Checks::answer(
        &program,
        "/orders",
        &serde_json::json!({"name": "", "tags": ["a", "b", "c"]}),
    );
    assert_eq!(
        rejected,
        serde_json::json!({"status": 422, "violations": [
            {"field": "name", "rule": "NotEmpty"},
            {"field": "name", "rule": "Length"},
            {"field": "level", "rule": "Required"},
            {"field": "tags", "rule": "Length"},
        ]})
    );

    let bounded = Checks::answer(
        &program,
        "/orders/direct",
        &serde_json::json!({"name": "   ", "level": 10}),
    );
    assert_eq!(
        bounded,
        serde_json::json!({"status": 422, "violations": [
            {"field": "name", "rule": "NotEmpty"},
            {"field": "level", "rule": "Max"},
        ]})
    );

    let direct = Checks::answer(
        &program,
        "/orders/direct",
        &serde_json::json!({"name": "abcdefgh", "level": 1}),
    );
    assert_eq!(direct["status"], 200, "{direct}");

    let product = Checks::answer(
        &program,
        "/orders/product",
        &serde_json::json!({"detail": {"id": 7}, "extra": true}),
    );
    assert_eq!(
        Checks::body(&product),
        serde_json::json!({"productId": 7, "kind": "product"})
    );
}

#[test]
fn checks_and_converters_are_refused_where_they_cannot_work() {
    let unchecked = Checks::CONTROLLER.replace(
        "OrderInput postDirect([[FromBody]] [[Check]] const OrderInput& order) { return order; }",
        "OrderSummary postDirect([[FromBody]] [[Check]] const OrderSummary& order) { return order; }",
    );
    let failure = Checks::refusal(&[], &unchecked);
    assert!(failure.contains("server/models in Yon"), "{failure}");

    let cases: [(&str, &str, &str, &str); 4] = [
        (
            "server/services/OrderService.hpp",
            "class [[Service]] OrderService",
            "class [[Service]] [[Checked]] OrderService",
            "go on a C++ model",
        ),
        (
            "server/converters/OrderConverter.hpp",
            "OrderSummary toSummary(const OrderInput& order) const;",
            "OrderSummary toSummary(const OrderInput& order) const { return OrderSummary{}; }",
            "A Converter has no implementation",
        ),
        (
            "server/converters/OrderConverter.hpp",
            "{\"target\":\"kind\",\"constant\":\"order\"}",
            "{\"target\":\"missing\",\"constant\":\"order\"}",
            "OrderSummary has no field missing",
        ),
        (
            "server/models/OrderInput.hpp",
            "struct [[Checked]] OrderInput",
            "struct OrderInput",
            "needs [[Checked]] on its model",
        ),
    ];
    for (file, from, to, expected) in cases {
        let failure = Checks::refusal(&[(file, from, to)], Checks::CONTROLLER);
        assert!(failure.contains(expected), "{to}: {failure}");
    }

    let root = Checks::project(&[]);
    std::fs::remove_file(root.path().join("server/converters/OrderConverter.hpp"))
        .expect("converter removed");
    let failure = CppYon::lower(
        Path::new(Checks::CONTROLLER_PATH),
        Checks::CONTROLLER,
        root.path(),
    )
    .err()
    .expect("a missing converter is refused")
    .to_string();
    assert!(
        failure.contains("[[Converter]] in server/converters"),
        "{failure}"
    );
}

#[test]
fn a_cpp_settings_model_takes_min_and_max_bounds() {
    let root = Checks::project(&[]);
    Roles::write(
        root.path(),
        "server/models/CatalogSettings.hpp",
        "#pragma once\n\nstruct [[Settings({\"prefix\":\"YON_CATALOG_\"})]] CatalogSettings {\n    [[Min(1)]] [[Max(100)]] int size = 20;\n};\n",
    );
    let program = CppYon::lower(
        Path::new(Checks::CONTROLLER_PATH),
        Checks::CONTROLLER,
        root.path(),
    )
    .expect("lowered")
    .source;
    assert!(program.contains("TacSettingsPlan<::CatalogSettings>"));
    assert!(!program.contains("[[Min(1)]]"));
    assert!(program.contains(r#"\"max\":100,\"min\":1"#), "{program}");
}
