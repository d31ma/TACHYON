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

#[test]
fn remotes_and_clients_aliases_compose_in_both_realms() {
    for realm in ["client", "server"] {
        for folder in ["remotes", "clients"] {
            let root = tempfile::tempdir().expect("project");
            let (role, environment) = if realm == "client" {
                ("Service", "tac")
            } else {
                ("RequestRoute", "yon")
            };
            let entry = format!("{realm}/services/OrdersService.js");
            let entry = if realm == "server" {
                String::from("server/controllers/OrdersController.js")
            } else {
                entry
            };
            let source = if role == "Service" {
                "@Service\nclass OrdersService { @Needs ordersClient; }\n"
            } else {
                Sources::CONTROLLER
            };
            Sources::write(root.path(), &entry, source);
            Sources::write(
                root.path(),
                &format!("{realm}/{folder}/OrdersClient.js"),
                "@HTTPClient\nclass OrdersClient {}\n",
            );
            let composed = assemble(root.path(), Path::new(&entry), source, environment)
                .expect("outbound role discovery")
                .expect("composed outbound class");
            assert!(composed.contains("class OrdersClient"), "{realm}/{folder}");
        }
    }
}

#[test]
fn remotes_and_clients_aliases_refuse_duplicate_type_names() {
    let root = Sources::project();
    for folder in ["remotes", "clients"] {
        Sources::write(
            root.path(),
            &format!("server/{folder}/OrdersClient.js"),
            "@HTTPClient\nclass OrdersClient {}\n",
        );
    }
    let error = assemble(
        root.path(),
        Path::new("server/controllers/OrdersController.js"),
        Sources::CONTROLLER,
        "yon",
    )
    .expect_err("conflicting identities")
    .to_string();
    assert!(error.contains("duplicate type name"), "{error}");
}

#[test]
fn a_one_segment_relative_project_root_opens_from_the_working_directory() {
    // Cargo runs tests from the crate directory, so `src` is a relative root with an empty parent.
    assert!(super::open_root(Path::new("src"), Path::new("src")).is_ok());
}
