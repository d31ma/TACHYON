#![allow(clippy::expect_used)]

use super::assemble;
use std::path::Path;

struct Sources;

impl Sources {
    const CONTROLLER: &str = "@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n@GetRoute\ngetOrders() {}\n}\n";
    const ERROR_HANDLER: &str = "@ErrorHandler\nclass OrdersErrorHandler {}\n";

    fn write(root: &Path, name: &str, source: &str) {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::write(
            root.path(),
            "server/controllers/OrdersController.js",
            Self::CONTROLLER,
        );
        Self::write(
            root.path(),
            "server/controllers/OrdersErrorHandler.js",
            Self::ERROR_HANDLER,
        );
        Self::write(
            root.path(),
            "server/errors/OrderNotFound.js",
            "class OrderNotFound extends Error {}\n",
        );
        root
    }
}

#[test]
fn controller_composes_error_handlers_and_plain_errors_but_not_other_controllers() {
    let root = Sources::project();
    Sources::write(
        root.path(),
        "server/controllers/OtherController.js",
        "@RequestRoute\n@Public\nclass OtherController {}\n",
    );
    Sources::write(
        root.path(),
        "server/errors/SpecificNotFound.js",
        "class SpecificNotFound extends OrderNotFound {}\n",
    );
    let source = assemble(
        root.path(),
        Path::new("server/controllers/OrdersController.js"),
        Sources::CONTROLLER,
        "yon",
    )
    .expect("assemble")
    .expect("composed sources");
    assert!(source.contains("class OrdersErrorHandler"));
    assert!(source.contains("class OrderNotFound"));
    assert!(!source.contains("class OtherController"));
    assert!(source.find("class OrderNotFound") < source.find("class SpecificNotFound"));
}

#[test]
fn worker_does_not_compose_global_controller_error_sources() {
    let root = Sources::project();
    let source = "class Handler { GET() {} }";
    Sources::write(root.path(), "server/workers/Job.js", source);
    let composed = assemble(
        root.path(),
        Path::new("server/workers/Job.js"),
        source,
        "yon",
    )
    .expect("worker");
    assert!(composed.is_none());
}

#[test]
fn error_type_names_cannot_duplicate_provider_names() {
    let root = Sources::project();
    Sources::write(
        root.path(),
        "server/errors/OrdersService.js",
        "class OrdersService extends Error {}\n",
    );
    Sources::write(
        root.path(),
        "server/services/OrdersService.js",
        "@Service\nclass OrdersService {}\n",
    );
    let result = assemble(
        root.path(),
        Path::new("server/controllers/OrdersController.js"),
        Sources::CONTROLLER,
        "yon",
    );
    assert!(result.is_err());
}

#[cfg(unix)]
#[test]
fn error_discovery_never_follows_a_source_symlink() {
    let root = Sources::project();
    std::os::unix::fs::symlink(
        "OrderNotFound.js",
        root.path().join("server/errors/Alias.js"),
    )
    .expect("symlink");
    let result = assemble(
        root.path(),
        Path::new("server/controllers/OrdersController.js"),
        Sources::CONTROLLER,
        "yon",
    );
    assert!(result.is_err());
}
