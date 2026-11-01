use super::role_tests::Roles;
use super::{CppYon, CppYonGatewayUnit};
use std::collections::BTreeMap;
use std::path::Path;

struct Shared;

impl Shared {
    const CONTROLLER_PATH: &'static str = "server/controllers/OrderController.hpp";
    const CONTROLLER: &'static str = r#"#pragma once
#include <string>
#include "../shared/OrderNames.hpp"

class [[RequestRoute("/orders")]] [[Public]] OrderController {
    [[Needs]] LabelService& labels;

public:
    [[GetRoute("/{item}")]]
    std::string getOrder([[FromPath]] std::string item) {
        return OrderNames::label(item) + labels.suffix();
    }
};
"#;
    const ORDER_NAMES_PATH: &'static str = "server/shared/OrderNames.hpp";
    const ORDER_NAMES: &'static str = r#"#pragma once
#include <string>
#include "PriceText.hpp"

class OrderNames {
public:
    static std::string label(const std::string& item) {
        return "order:" + item + PriceText::currency();
    }
};
"#;
    const PRICE_TEXT_PATH: &'static str = "server/shared/money/PriceText.hpp";
    const PRICE_TEXT: &'static str = r#"#pragma once
#include <string>

class PriceText {
public:
    static std::string currency() { return "/EUR"; }
};
"#;
    const FILES: [(&'static str, &'static str); 5] = [
        (Self::ORDER_NAMES_PATH, Self::ORDER_NAMES),
        (Self::PRICE_TEXT_PATH, Self::PRICE_TEXT),
        (
            "server/models/Order.hpp",
            "#pragma once\n#include <string>\n\nstruct Order {\n    std::string item;\n    int count = 1;\n};\n",
        ),
        (
            "server/shared/OrderTotals.hpp",
            "#pragma once\n\nclass OrderTotals {\npublic:\n    static int count(const Order& order) { return order.count * 2; }\n};\n",
        ),
        (
            "server/services/LabelService.hpp",
            "#pragma once\n#include <string>\n\nclass [[Service]] LabelService {\npublic:\n    std::string suffix() { return OrderNames::label(\"x\").substr(0, 1); }\n};\n",
        ),
    ];
    const UNIT_PATH: &'static str = "server/tests/OrderControllerTest.hpp";
    const UNIT: &'static str = r#"#pragma once

class [[Test(OrderTestSetup)]] OrderControllerTest {
public:
    void testLabelsAnOrder() {
        Assert::equals(std::string("order:tea/EUR"), OrderNames::label("tea"));
    }

    void testTheControllerUsesTheHelper() {
        OrderController controller = construct<OrderController>();
        Assert::equals(std::string("order:tea/EURo"), controller.getOrder("tea"));
    }
};
"#;
    const SETUP: (&'static str, &'static str) = (
        "server/tests/OrderTestSetup.hpp",
        "#pragma once\n\nclass [[TestSetup]] OrderTestSetup {\npublic:\n    static BehaviorAdapters configure(const BehaviorExecution&) {\n        return BehaviorAdapters();\n    }\n};\n",
    );

    fn project(changes: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in Self::FILES.iter().chain(changes) {
            Roles::write(root.path(), relative, source);
        }
        Roles::write(root.path(), Self::CONTROLLER_PATH, Self::CONTROLLER);
        root
    }

    fn program(root: &Path) -> String {
        CppYon::lower(Path::new(Self::CONTROLLER_PATH), Self::CONTROLLER, root)
            .expect("lowered")
            .source
    }

    fn refusal(root: &Path) -> String {
        CppYon::lower(Path::new(Self::CONTROLLER_PATH), Self::CONTROLLER, root)
            .err()
            .expect("the project is refused")
            .to_string()
    }

    fn compiler_ready() -> bool {
        let found = Roles::compiler_available();
        assert!(
            found || std::env::var_os("CI").is_none(),
            "clang++ is installed in CI, so this test must run there instead of skipping"
        );
        found
    }
}

#[test]
fn a_cpp_shared_class_is_written_before_the_classes_that_use_it() {
    let root = Shared::project(&[]);
    let program = Shared::program(root.path());
    let price = program.find("class PriceText {").expect("PriceText");
    let names = program.find("class OrderNames {").expect("OrderNames");
    let service = program.find("LabelService {").expect("LabelService");
    let controller = program.find("OrderController {").expect("OrderController");
    assert!(
        [price < names, names < service, service < controller] == [true; 3],
        "{program}"
    );
    assert!(program.contains("#line 1 \"server/shared/money/PriceText.hpp\""));
    assert!(!program.contains("#include \"PriceText.hpp\""), "{program}");
}

#[test]
fn a_cpp_shared_class_comes_after_the_server_models_it_uses() {
    let root = Shared::project(&[]);
    let program = Shared::program(root.path());
    let model = program.find("struct Order {").expect("Order");
    let totals = program.find("class OrderTotals {").expect("OrderTotals");
    assert!(model < totals, "{program}");
}

#[test]
fn a_cpp_gateway_unit_gets_the_shared_classes_before_its_target() {
    let root = Shared::project(&[]);
    let unit = CppYonGatewayUnit::lower(
        root.path(),
        Path::new("server/gateways/labels/yon.hpp"),
        "LabelService",
        &BTreeMap::new(),
    )
    .expect("the gateway unit is lowered");
    let names = unit.find("class OrderNames {").expect("OrderNames");
    let target = unit.find("LabelService {").expect("LabelService");
    assert!(names < target, "{unit}");
}

#[test]
fn a_name_in_a_comment_or_a_string_does_not_order_cpp_shared_classes() {
    let root = Shared::project(&[(
        Shared::PRICE_TEXT_PATH,
        &Shared::PRICE_TEXT.replace(
            "return \"/EUR\";",
            "return \"/EUR\"; // OrderNames\n    static const char* note() { return \"OrderNames\"; }",
        ),
    )]);
    let program = Shared::program(root.path());
    assert!(program.contains("class PriceText {"), "{program}");
}

#[test]
fn two_cpp_shared_classes_with_one_name_are_refused_with_ty1310() {
    let root = Shared::project(&[(
        "server/shared/copy/OrderNames.hpp",
        "#pragma once\n\nclass OrderNames {};\n",
    )]);
    let failure = Shared::refusal(root.path());
    assert!(failure.contains("TY1310"), "{failure}");
    assert!(
        failure.contains("already declares a class with this name"),
        "{failure}"
    );
}

#[test]
fn a_cpp_shared_class_named_like_a_service_is_refused_with_ty1310() {
    let root = Shared::project(&[(
        "server/shared/LabelService.hpp",
        "#pragma once\n\nclass LabelService {};\n",
    )]);
    let failure = Shared::refusal(root.path());
    assert!(failure.contains("TY1310"), "{failure}");
    assert!(
        failure.contains("has the name of a controller"),
        "{failure}"
    );
}

#[test]
fn a_cpp_shared_class_that_uses_a_service_is_refused_with_ty1310() {
    let root = Shared::project(&[(
        Shared::PRICE_TEXT_PATH,
        &Shared::PRICE_TEXT.replace("return \"/EUR\";", "return LabelService().suffix();"),
    )]);
    let failure = Shared::refusal(root.path());
    assert!(failure.contains("TY1310"), "{failure}");
    assert!(failure.contains("does not use a controller"), "{failure}");
}

#[cfg(unix)]
#[test]
fn a_cpp_shared_file_name_with_a_quote_is_refused_with_ty1310() {
    let root = Shared::project(&[(
        "server/shared/Odd\"Name.hpp",
        "#pragma once\n\nclass OddName {};\n",
    )]);
    let failure = Shared::refusal(root.path());
    assert!(failure.contains("TY1310"), "{failure}");
    assert!(
        failure.contains("without quotes or control characters"),
        "{failure}"
    );
}

#[test]
fn a_cpp_shared_class_with_a_tachyon_marker_is_refused_with_ty1310() {
    let root = Shared::project(&[(
        Shared::PRICE_TEXT_PATH,
        &Shared::PRICE_TEXT.replace("class PriceText", "class [[Service]] PriceText"),
    )]);
    let failure = Shared::refusal(root.path());
    assert!(failure.contains("TY1310"), "{failure}");
    assert!(failure.contains("without a Tachyon marker"), "{failure}");
    assert!(
        failure.contains("server/shared/money/PriceText.hpp"),
        "{failure}"
    );
}

#[test]
fn a_cpp_shared_method_marker_is_refused_with_ty1310() {
    let root = Shared::project(&[(
        Shared::PRICE_TEXT_PATH,
        &Shared::PRICE_TEXT.replace("static std::string", "[[Cached]] static std::string"),
    )]);
    let failure = Shared::refusal(root.path());
    assert!(failure.contains("TY1310"), "{failure}");
}

#[test]
fn a_cpp_shared_class_keeps_its_standard_attributes() {
    let root = Shared::project(&[(
        Shared::PRICE_TEXT_PATH,
        &Shared::PRICE_TEXT.replace("static std::string", "[[nodiscard]] static std::string"),
    )]);
    let program = Shared::program(root.path());
    assert!(
        program.contains("[[nodiscard]] static std::string currency()"),
        "{program}"
    );
}

#[test]
fn cpp_shared_classes_that_use_each_other_are_refused_with_ty1310() {
    let root = Shared::project(&[(
        Shared::PRICE_TEXT_PATH,
        &Shared::PRICE_TEXT.replace(
            "return \"/EUR\";",
            "return OrderNames::label(\"\").empty() ? \"\" : \"/EUR\";",
        ),
    )]);
    let failure = Shared::refusal(root.path());
    assert!(failure.contains("TY1310"), "{failure}");
    assert!(failure.contains("in a circle"), "{failure}");
}

#[test]
fn the_layout_check_reads_a_cpp_shared_class_with_the_cpp_reader() {
    let checked = crate::annotation_layout::validate(
        Path::new(Shared::ORDER_NAMES_PATH),
        Shared::ORDER_NAMES,
    );
    assert!(checked.is_ok(), "{checked:?}");
}

#[test]
fn the_layout_check_refuses_a_marked_cpp_shared_class_with_ty1310() {
    let failure = crate::annotation_layout::validate(
        Path::new(Shared::ORDER_NAMES_PATH),
        &Shared::ORDER_NAMES.replace("class OrderNames", "class [[Delegate]] OrderNames"),
    )
    .expect_err("a marked shared class is refused")
    .to_string();
    assert!(failure.contains("TY1310"), "{failure}");
}

#[test]
fn the_layout_check_refuses_a_cpp_shared_file_with_a_second_class() {
    let failure = crate::annotation_layout::validate(
        Path::new(Shared::ORDER_NAMES_PATH),
        &format!("{}\nclass Other {{}};\n", Shared::ORDER_NAMES),
    )
    .expect_err("a second class is refused");
    assert_eq!(failure.diagnostics()[0].code.number(), 1371, "{failure}");
}

#[cfg(unix)]
#[test]
fn a_symlinked_cpp_shared_file_is_refused_with_ty1310() {
    let outside = tempfile::tempdir().expect("outside");
    Roles::write(outside.path(), "PriceText.hpp", Shared::PRICE_TEXT);
    let root = Shared::project(&[]);
    std::fs::remove_file(root.path().join(Shared::PRICE_TEXT_PATH)).expect("remove");
    std::os::unix::fs::symlink(
        outside.path().join("PriceText.hpp"),
        root.path().join(Shared::PRICE_TEXT_PATH),
    )
    .expect("file link");
    let failure = Shared::refusal(root.path());
    assert!(failure.contains("TY1310"), "{failure}");
    assert!(failure.contains("symlink"), "{failure}");
}

#[cfg(unix)]
#[test]
fn a_symlinked_cpp_shared_folder_is_refused_with_ty1310() {
    let outside = tempfile::tempdir().expect("outside");
    Roles::write(outside.path(), "PriceText.hpp", Shared::PRICE_TEXT);
    let root = Shared::project(&[]);
    std::fs::remove_dir_all(root.path().join("server/shared/money")).expect("remove");
    std::os::unix::fs::symlink(outside.path(), root.path().join("server/shared/money"))
        .expect("folder link");
    let failure = Shared::refusal(root.path());
    assert!(failure.contains("TY1310"), "{failure}");
    assert!(failure.contains("(server/shared/money)"), "{failure}");
}

#[test]
fn a_cpp_controller_with_shared_classes_compiles_and_answers() {
    if !Shared::compiler_ready() {
        return;
    }
    let root = Shared::project(&[]);
    let lowered = CppYon::lower(
        Path::new(Shared::CONTROLLER_PATH),
        Shared::CONTROLLER,
        root.path(),
    )
    .expect("lowered");
    let route = lowered.routes[0].pattern.clone().expect("route pattern");
    let frame = serde_json::json!({"method": "GET", "route": route, "arguments": ["tea"]});
    let (answer, errors) = Roles::run(&lowered.source, &frame.to_string());
    assert_eq!(answer["status"], 200, "{answer} {errors}");
    assert_eq!(answer["body"], "\"order:tea/EURo\"", "{answer}");
}

#[test]
fn a_cpp_test_unit_calls_a_server_shared_helper() {
    if !Shared::compiler_ready() {
        return;
    }
    let root = Shared::project(&[Shared::SETUP, (Shared::UNIT_PATH, Shared::UNIT)]);
    let lowered =
        CppYon::lower(Path::new(Shared::UNIT_PATH), Shared::UNIT, root.path()).expect("lowered");
    let (answer, errors) = Roles::run(&lowered.source, r#"{"method":"POST","route":"/"}"#);
    let report: serde_json::Value =
        serde_json::from_str(answer["body"].as_str().expect("body")).expect("report");
    assert_eq!(
        report["cases"],
        serde_json::json!([
            {"called": [], "name": "testLabelsAnOrder", "passed": true},
            {
                "called": ["OrderController.getOrder", "LabelService.suffix"],
                "name": "testTheControllerUsesTheHelper",
                "passed": true
            }
        ]),
        "{report} {errors}"
    );
}

#[cfg(unix)]
#[test]
fn a_cpp_shared_folder_that_links_back_to_itself_is_refused_with_ty1310() {
    let root = Shared::project(&[]);
    std::os::unix::fs::symlink(
        root.path().join("server/shared"),
        root.path().join("server/shared/again"),
    )
    .expect("loop link");
    let failure = Shared::refusal(root.path());
    assert!(failure.contains("TY1310"), "{failure}");
    assert!(failure.contains("(server/shared/again)"), "{failure}");
}
