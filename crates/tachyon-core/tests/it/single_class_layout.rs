#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use tachyon_core::{HandlerSource, ProjectDiscovery};

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("parent")).expect("directories");
    fs::write(path, text).expect("source");
}

fn write_home_controller(root: &Path) {
    write(
        root,
        "server/controllers/HomeController.ts",
        "@RequestRoute(\"/\")\n@Public\nexport class HomeController {\n  @GetRoute\n  get() { return {}; }\n}\n",
    );
}

#[test]
fn extra_handler_declarations_fail_in_project_and_direct_discovery() {
    for (path, source) in [
        (
            "server/controllers/OrdersController.js",
            "@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n  @GetRoute\n  getOrders() { return 1; }\n}\nconst extra = 2;",
        ),
        (
            "server/workers/nightly.py",
            "class Handler:\n    @staticmethod\n    def POST(request):\n        return {}\n\ndef extra():\n    return 1\n",
        ),
        (
            "middleware.js",
            "export class Middleware { static before() {} }\nclass Extra {}",
        ),
        (
            "server/controllers/OrdersController.go",
            "// @RequestRoute(\"/orders\")\n// @Public\ntype OrdersController struct{}\n\n// @GetRoute\nfunc (controller *OrdersController) GetOrders() any { return nil }\n\ntype Extra struct{}\n",
        ),
        (
            "server/workers/nightly.go",
            "type Handler struct{}\n\nfunc (Handler) POST() any { return nil }\n\nvar extra = 1\n",
        ),
        (
            "middleware.go",
            "type Middleware struct{}\n\nfunc init() {}\n",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        write_home_controller(root.path());
        write(root.path(), path, source);
        let discovered = ProjectDiscovery::discover(root.path()).expect_err("authored layout");
        assert!(
            discovered.to_string().contains("TY1310"),
            "{path}: {discovered}"
        );
        let invoked =
            HandlerSource::discover(root.path(), path).expect_err("direct authored layout");
        assert!(invoked.to_string().contains("TY1310"), "{path}: {invoked}");
    }
}

#[test]
fn an_unused_annotated_role_is_validated_before_execution() {
    let root = tempfile::tempdir().expect("project");
    write_home_controller(root.path());
    write(
        root.path(),
        "server/services/MoneyService.js",
        "@Service\nclass MoneyService {}\nfunction extra() { throw Error('must never run'); }",
    );
    let failure = ProjectDiscovery::discover(root.path()).expect_err("unused role still checked");
    assert!(failure.to_string().contains("TY1310"), "{failure}");
    assert!(failure.to_string().contains("MoneyService.js"), "{failure}");
}

#[test]
fn a_controller_beside_an_external_program_remains_valid() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/controllers/MoneyController.js",
        "@RequestRoute(\"/money\")\n@Public\nclass MoneyController {\n  @GetRoute\n  getMoney() { return 42; }\n}\n",
    );
    write(
        root.path(),
        "server/helpers/report.py",
        "import json\ndef calculate():\n    return 42\nprint(json.dumps(calculate()))\n",
    );
    ProjectDiscovery::discover(root.path()).expect("controller and external program");
    HandlerSource::discover(root.path(), "server/controllers/MoneyController.js")
        .expect("composed controller unit");
}

#[test]
fn same_line_extras_do_not_hide_behind_imports_or_type_bodies() {
    for source in [
        "import { format } from '/shared/scripts/format.js'; const hidden = 1;\n@Service\nclass MoneyService {}",
        "@Service\nclass MoneyService {} class Hidden {}",
        "@Service\nclass MoneyService {} ; (() => 1)();",
    ] {
        let root = tempfile::tempdir().expect("project");
        write_home_controller(root.path());
        write(root.path(), "server/services/MoneyService.js", source);
        let failure = ProjectDiscovery::discover(root.path()).expect_err("same-line extra");
        assert!(failure.to_string().contains("TY1310"), "{failure}");
    }
}

#[test]
fn a_handler_class_in_root_middleware_fails_with_ty1310() {
    let root = tempfile::tempdir().expect("project");
    write_home_controller(root.path());
    write(
        root.path(),
        "middleware.php",
        "<?php\nfinal class Handler\n{\n    public static function GET(): void\n    {\n        http_response_code(204);\n    }\n}\n",
    );
    let discovered = ProjectDiscovery::discover(root.path()).expect_err("Handler in middleware");
    let invoked =
        HandlerSource::discover(root.path(), "middleware.php").expect_err("direct middleware");
    for failure in [discovered, invoked] {
        let text = failure.to_string();
        assert!(text.contains("TY1310"), "{text}");
        assert!(
            text.contains(
                "root middleware declares class Middleware with before(request) and after(request)"
            ),
            "{text}"
        );
    }
}

#[test]
fn a_controller_file_without_a_controller_fails_project_discovery() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/controllers/MoneyController.js",
        "// No controller here.\n",
    );
    let failure = ProjectDiscovery::discover(root.path()).expect_err("file without controller");
    let text = failure.to_string();
    assert!(text.contains("TY2015"), "{text}");
    assert!(
        text.contains("'server/controllers/MoneyController.js' does not declare"),
        "{text}"
    );
}

#[test]
fn a_go_controller_file_without_a_controller_fails_with_ty2015() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/controllers/MoneyController.go",
        "package main\n",
    );
    let discovered = ProjectDiscovery::discover(root.path()).expect_err("Go file");
    let invoked = HandlerSource::discover(root.path(), "server/controllers/MoneyController.go")
        .expect_err("Go file");
    for failure in [discovered, invoked] {
        let text = failure.to_string();
        assert!(text.contains("TY2015"), "{text}");
        assert!(
            text.contains("'server/controllers/MoneyController.go' does not declare"),
            "{text}"
        );
    }
}

#[test]
fn a_go_handler_type_in_root_middleware_fails_with_ty1310() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "middleware.go",
        "type Handler struct{}\n\nfunc (Handler) GET() any { return nil }\n",
    );
    let discovered = ProjectDiscovery::discover(root.path()).expect_err("Handler in middleware");
    let invoked =
        HandlerSource::discover(root.path(), "middleware.go").expect_err("direct middleware");
    for failure in [discovered, invoked] {
        let text = failure.to_string();
        assert!(text.contains("TY1310"), "{text}");
        assert!(
            text.contains("declares type Middleware with Before(request) and After(request)"),
            "{text}"
        );
    }
}

#[test]
fn an_annotated_go_role_follows_the_role_folder_rules() {
    let root = tempfile::tempdir().expect("project");
    write_home_controller(root.path());
    write(
        root.path(),
        "server/services/MoneyService.go",
        "// @Service\ntype MoneyService struct{}\n",
    );
    ProjectDiscovery::discover(root.path()).expect("a Go service in its role folder");
    write(
        root.path(),
        "server/helpers/LedgerService.go",
        "// @Service\ntype LedgerService struct{}\n",
    );
    let failure = ProjectDiscovery::discover(root.path()).expect_err("misplaced Go role");
    let text = failure.to_string();
    assert!(text.contains("TY1310"), "{text}");
    assert!(text.contains("server/services"), "{text}");
}
