use super::{CppYon, CppYonController, CppYonModels};
use crate::route_types::{Model, Shape, TypeText};
use crate::yon::plan::ParameterSource;
use crate::yon::route_access::RouteAccess;
use std::path::Path;

struct Sample;

impl Sample {
    const PATH: &'static str = "server/controllers/OrderController.hpp";
    const CONTROLLER: &'static str = r#"#pragma once
#include <string>

class [[RequestRoute("/orders")]] [[SignedIn]] OrderController {
public:
    [[GetRoute("/{id}")]] [[Public]]
    Order getOrder([[FromPath]] int id, [[FromQuery("view")]] const std::string& view) {
        return Order{};
    }

    [[PostRoute]] [[Allow(Admin, Editor)]]
    void postOrder([[FromBody]] const Order& order, [[FromHeader("X-Tenant")]] std::string tenant) {}

private:
    [[DeleteRoute("/{id}")]]
    std::optional<Order> deleteOrder([[FromPath]] long id) { return std::nullopt; }
};
"#;

    fn read(source: &str) -> Result<CppYonController, crate::Failure> {
        CppYonController::read(Path::new(Self::PATH), source)
    }

    fn refusal(from: &str, to: &str) -> String {
        let source = Self::CONTROLLER.replace(from, to);
        assert_ne!(source, Self::CONTROLLER, "the sample changes");
        Self::read(&source)
            .err()
            .expect("the controller is refused")
            .to_string()
    }
}

#[test]
fn a_controller_lists_its_routes_with_their_parameters_and_access() {
    let controller = Sample::read(Sample::CONTROLLER).expect("controller");
    let routes = &controller.routes;
    assert_eq!(routes.len(), 3);
    assert_eq!(routes[0].verb, "GET");
    assert_eq!(routes[0].method, "getOrder");
    assert_eq!(routes[0].pattern.as_deref(), Some("/orders/_id"));
    assert_eq!(routes[0].returns, "Order");
    assert_eq!(routes[0].access, RouteAccess::Public);
    assert_eq!(routes[0].parameters[0].source, ParameterSource::Path);
    assert_eq!(routes[0].parameters[0].type_text, "int");
    assert_eq!(routes[0].parameters[1].source, ParameterSource::Query);
    assert_eq!(routes[0].parameters[1].type_text, "std::string");
    assert_eq!(routes[1].pattern.as_deref(), Some("/orders"));
    assert_eq!(routes[1].returns, "void");
    assert_eq!(
        routes[1].access,
        RouteAccess::Allow(vec![String::from("Admin"), String::from("Editor")])
    );
    assert_eq!(routes[1].parameters[1].name, "X-Tenant");
    assert_eq!(routes[1].parameters[1].code_name, "tenant");
    assert_eq!(routes[2].verb, "DELETE");
    assert_eq!(routes[2].access, RouteAccess::SignedIn);
}

#[test]
fn the_program_dispatches_each_route_and_lets_it_reach_private_methods() {
    let root = tempfile::tempdir().expect("root");
    let lowered =
        CppYon::lower(Path::new(Sample::PATH), Sample::CONTROLLER, root.path()).expect("lowered");
    let program = lowered.source;
    assert!(program.contains("friend class TachyonYonRoutes;"));
    assert!(program.contains("#line 1 \"server/controllers/OrderController.hpp\""));
    assert!(program.contains(
        "if (verb == \"GET\" && route == \"/orders/_id\") {\n            return tachyon::yon::YonAnswer::of(controller.getOrder(frame.argument<int>(0), frame.argument<std::string>(1)));"
    ));
    assert!(program.contains(
        "controller.postOrder(frame.argument<Order>(0), frame.argument<std::string>(1));\n            return tachyon::yon::YonAnswer::empty(204);"
    ));
    assert!(program.contains("return tachyon::yon::YonMain::run<TachyonYonRoutes>();"));
    assert!(!program.contains("[[GetRoute"));
    assert!(!program.contains("#pragma once"));
    assert_eq!(lowered.routes.len(), 3);
}

#[test]
fn route_methods_follow_the_shared_route_rules() {
    let cases = [
        (
            "Order getOrder([[FromPath]] int id",
            "static Order getOrder([[FromPath]] int id",
            "is static",
        ),
        ("Order getOrder(", "Order findOrder(", "starts with 'get'"),
        (
            "[[FromPath]] int id, [[FromQuery",
            "int id, [[FromQuery",
            "[[FromPath]], [[FromQuery]]",
        ),
        (
            "[[PostRoute]] [[Allow(Admin, Editor)]]\n    void postOrder(",
            "[[GetRoute(\"/new\")]] [[Allow(Admin, Editor)]]\n    void getNew(",
            "a GET request has no body",
        ),
        (
            "[[DeleteRoute(\"/{id}\")]]",
            "[[DeleteRoute]]",
            "has no {id} part",
        ),
        (
            "[[RequestRoute(\"/orders\")]]",
            "[[RequestRoute(/orders)]]",
            "takes one path in quotes",
        ),
        (
            "[[DeleteRoute(\"/{id}\")]]\n    std::optional<Order> deleteOrder(",
            "[[GetRoute(\"/{id}\")]]\n    std::optional<Order> getAgain(",
            "Two methods answer",
        ),
        (
            "[[FromQuery(\"view\")]]",
            "[[FromQuery(\"a b\")]]",
            "takes one name in quotes",
        ),
    ];
    for (from, to, expected) in cases {
        let failure = Sample::refusal(from, to);
        assert!(failure.contains(expected), "{to}: {failure}");
    }
}

#[test]
fn unsupported_markers_name_the_later_step() {
    let failure = Sample::refusal("[[Public]]\n", "[[Public]] [[ResponseStatus(201)]]\n");
    assert!(failure.contains("C++ gets it in a later step"), "{failure}");
    let failure = Sample::refusal("[[SignedIn]]", "[[SignedIn]] [[Needs]]");
    assert!(failure.contains("[[Needs]]"), "{failure}");
}

#[test]
fn a_cpp_model_becomes_a_route_record() {
    let models = CppYonModels::read(
        "struct Order {\n    std::string item;\n    int count = 1;\n    std::optional<std::string> note;\n    std::vector<Line> lines;\n    static int made;\n};\n",
    );
    let Some((name, Model::Record(record))) = models.first() else {
        panic!("a record model: {models:?}");
    };
    assert_eq!(name, "Order");
    let fields: Vec<(&str, bool)> = record
        .fields
        .iter()
        .map(|field| (field.name.as_str(), field.required))
        .collect();
    assert_eq!(
        fields,
        [
            ("item", true),
            ("count", false),
            ("note", false),
            ("lines", true)
        ]
    );
    assert_eq!(
        record.fields[3].shape,
        Shape::List(Box::new(Shape::Model(String::from("Line"))))
    );
    let models = CppYonModels::read("enum class Size { Small, Large };\n");
    assert_eq!(models, [(String::from("Size"), Model::Alias(Shape::Text))]);
}

#[test]
fn cpp_type_names_have_route_shapes() {
    let cases = [
        ("int", Shape::Integer),
        ("long long", Shape::Integer),
        ("std::int64_t", Shape::Integer),
        ("double", Shape::Number),
        ("bool", Shape::Boolean),
        ("std::string", Shape::Text),
        ("void", Shape::Any),
        ("tachyon::json::JsonNode", Shape::Any),
        ("std::map<std::string, int>", Shape::Any),
        (
            "std::optional<int>",
            Shape::Optional(Box::new(Shape::Integer)),
        ),
        (
            "std::vector<Order>",
            Shape::List(Box::new(Shape::Model(String::from("Order")))),
        ),
    ];
    for (text, shape) in cases {
        assert_eq!(TypeText::shape("hpp", text), Ok(shape), "{text}");
    }
}
