use super::CppYon;
use std::path::Path;

struct Sample;

impl Sample {
    const PATH: &'static str = "middleware.hpp";
    const MIDDLEWARE: &'static str = r"#pragma once
#include <optional>

class Middleware {
public:
    std::optional<tachyon::http::HttpResponseMessage> before(const tachyon::http::HttpRequestMessage& request) {
        return std::nullopt;
    }
};
";

    fn refusal(from: &str, to: &str) -> String {
        let source = Self::MIDDLEWARE.replace(from, to);
        assert_ne!(source, Self::MIDDLEWARE, "the sample changes");
        let root = tempfile::tempdir().expect("root");
        CppYon::lower(Path::new(Self::PATH), &source, root.path())
            .err()
            .expect("the middleware is refused")
            .to_string()
    }
}

#[test]
fn root_middleware_dispatches_its_phases_by_operation_and_passes_a_missing_one() {
    let root = tempfile::tempdir().expect("root");
    let lowered =
        CppYon::lower(Path::new(Sample::PATH), Sample::MIDDLEWARE, root.path()).expect("lowered");
    let program = lowered.source;
    assert!(program.contains("std::string verb = frame.text(\"operation\");"));
    assert!(program.contains(
        "if (verb == \"middleware.before\") {\n            auto result = controller.before(tachyon::yon::YonRequest::of(frame));"
    ));
    assert!(!program.contains("middleware.after"));
    assert!(program.contains("return tachyon::yon::YonAnswer::empty(204);\n    }"));
    assert!(lowered.routes.is_empty());
}

#[test]
fn root_middleware_is_one_plain_class_with_answering_phases() {
    let cases = [
        (
            "class Middleware {",
            "class Gatekeeper {",
            "one `class Middleware`",
        ),
        (
            "class Middleware {",
            "class [[Service]] Middleware {",
            "takes no markers",
        ),
        (
            "std::optional<tachyon::http::HttpResponseMessage> before",
            "std::optional<int> before",
            "returns `std::optional<tachyon::http::HttpResponseMessage>`",
        ),
        (
            "before(const tachyon::http::HttpRequestMessage& request)",
            "before(int count)",
            "The request parameter may be left out",
        ),
    ];
    for (from, to, expected) in cases {
        let message = Sample::refusal(from, to);
        assert!(message.contains("TY2015"), "{to}: {message}");
        assert!(message.contains(expected), "{to}: {message}");
    }
}
