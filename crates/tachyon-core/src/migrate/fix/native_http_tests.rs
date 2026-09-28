use super::{MigrationFix, MigrationFixPlan};
use std::fs;
use std::path::Path;

struct UnusedRequest {
    relative: &'static str,
    source: &'static str,
    expected: &'static str,
}

impl UnusedRequest {
    const ALL: [Self; 6] = [
        Self {
            relative: "server/controllers/OrdersController.java",
            source: "@RequestRoute(\"/orders\")\nclass OrdersController {\n    @GetRoute\n    Object getOrders(YonRequest request) {\n        return java.util.List.of();\n    }\n}\n",
            expected: "@RequestRoute(\"/orders\")\nclass OrdersController {\n    @GetRoute\n    Object getOrders() {\n        return java.util.List.of();\n    }\n}\n",
        },
        Self {
            relative: "server/controllers/OrdersController.kt",
            source: "@RequestRoute(\"/orders\")\nclass OrdersController {\n    @GetRoute\n    fun getOrders(request: YonRequest): Any? = listOf<Any>()\n}\n",
            expected: "@RequestRoute(\"/orders\")\nclass OrdersController {\n    @GetRoute\n    fun getOrders(): Any? = listOf<Any>()\n}\n",
        },
        Self {
            relative: "server/controllers/OrdersController.cs",
            source: "[RequestRoute(\"/orders\")]\nclass OrdersController {\n    [GetRoute]\n    public object GetOrders(YonRequest request) => new object[0];\n}\n",
            expected: "[RequestRoute(\"/orders\")]\nclass OrdersController {\n    [GetRoute]\n    public object GetOrders() => new object[0];\n}\n",
        },
        Self {
            relative: "server/controllers/OrdersController.rs",
            source: "#[RequestRoute(\"/orders\")]\nstruct OrdersController {}\nimpl OrdersController {\n    #[GetRoute]\n    fn get_orders(&self, _request: &YonRequest) -> Json { Json::Null }\n}\n",
            expected: "#[RequestRoute(\"/orders\")]\nstruct OrdersController {}\nimpl OrdersController {\n    #[GetRoute]\n    fn get_orders(&self) -> Json { Json::Null }\n}\n",
        },
        Self {
            relative: "server/controllers/OrdersController.dart",
            source: "@RequestRoute(\"/orders\")\nclass OrdersController {\n  @GetRoute\n  Object? getOrders(YonRequest request) => [];\n}\n",
            expected: "@RequestRoute(\"/orders\")\nclass OrdersController {\n  @GetRoute\n  Object? getOrders() => [];\n}\n",
        },
        Self {
            relative: "server/controllers/OrdersController.go",
            source: "// @RequestRoute(\"/orders\")\ntype OrdersController struct{}\n\n// @GetRoute\nfunc (controller *OrdersController) GetOrders(\n\trequest *YonRequest,\n) any {\n\treturn []any{}\n}\n",
            expected: "// @RequestRoute(\"/orders\")\ntype OrdersController struct{}\n\n// @GetRoute\nfunc (controller *OrdersController) GetOrders() any {\n\treturn []any{}\n}\n",
        },
    ];
}

fn planned(relative: &str, source: &str) -> (tempfile::TempDir, MigrationFixPlan) {
    let root = tempfile::tempdir().expect("project");
    let path = root.path().join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    fs::write(&path, source).expect("file");
    let plan = MigrationFix::plan(root.path()).expect("plan");
    (root, plan)
}

#[test]
fn an_unused_yon_request_parameter_is_removed_in_every_typed_language() {
    for case in UnusedRequest::ALL {
        let (root, plan) = planned(case.relative, case.source);
        let text = plan.to_text(false);
        assert!(
            text.contains("Removed the unused YonRequest parameter"),
            "{}: {text}",
            case.relative
        );
        assert_eq!(plan.manual_step_count(), 0, "{}: {text}", case.relative);
        plan.write(root.path()).expect("write");
        let written = fs::read_to_string(root.path().join(case.relative)).expect("file");
        assert_eq!(written, case.expected, "{}", case.relative);
    }
}

#[test]
fn a_request_that_is_read_or_a_yon_response_is_a_manual_step_naming_the_native_type() {
    let cases = [
        (
            "server/controllers/OrdersController.go",
            "// @RequestRoute(\"/orders\")\ntype OrdersController struct{}\n\n// @GetRoute\nfunc (controller *OrdersController) GetOrders(request *YonRequest) any {\n\treturn request.Method()\n}\n",
            "*http.Request from net/http",
        ),
        (
            "middleware.cs",
            "sealed class Middleware {\n    public static YonResponse before() => YonResponse.Empty(204);\n}\n",
            "HttpResponseMessage",
        ),
    ];
    for (relative, source, native) in cases {
        let (root, plan) = planned(relative, source);
        let text = plan.to_text(false);
        assert_eq!(plan.manual_step_count(), 1, "{relative}: {text}");
        assert!(text.contains(native), "{relative}: {text}");
        assert!(!Path::new(&root.path().join(relative)).is_dir());
    }
}
