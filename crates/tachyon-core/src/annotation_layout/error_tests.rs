use super::validate;
use std::path::Path;

#[test]
fn error_handlers_use_controller_folder_and_error_handler_suffix() {
    let source = "@ErrorHandler\nclass OrdersErrorHandler {}";
    assert!(
        validate(
            Path::new("server/controllers/OrdersErrorHandler.ts"),
            source
        )
        .is_ok()
    );
    for path in [
        "client/controllers/OrdersErrorHandler.ts",
        "server/services/OrdersErrorHandler.ts",
        "server/controllers/OrdersController.ts",
        "server/controllers/nested/OrdersErrorHandler.ts",
    ] {
        assert!(validate(Path::new(path), source).is_err(), "{path}");
    }
}

#[test]
fn error_sources_hold_one_matching_native_type_without_a_role() {
    let sources = [
        ("ts", "class OrderNotFound extends Error {}"),
        ("js", "class OrderNotFound extends Error {}"),
        ("py", "class OrderNotFound(Exception):\n    pass\n"),
        ("php", "<?php\nclass OrderNotFound extends Exception {}"),
        ("java", "class OrderNotFound extends RuntimeException {}"),
        ("kt", "class OrderNotFound : RuntimeException() {}"),
        ("cs", "class OrderNotFound : System.Exception {}"),
        ("dart", "class OrderNotFound implements Exception {}"),
        (
            "go",
            "package main\ntype OrderNotFound struct{}\nfunc (error *OrderNotFound) Error() string { return \"missing\" }",
        ),
        (
            "rs",
            "struct OrderNotFound;\nimpl std::fmt::Display for OrderNotFound { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, \"missing\") } }\nimpl std::error::Error for OrderNotFound {}",
        ),
    ];
    for (extension, source) in sources {
        let path = format!("server/errors/OrderNotFound.{extension}");
        assert!(validate(Path::new(&path), source).is_ok(), "{path}");
        let wrong = format!("server/errors/Other.{extension}");
        assert!(validate(Path::new(&wrong), source).is_err(), "{wrong}");
    }
}

#[test]
fn error_sources_do_not_weaken_plain_model_or_load_time_rules() {
    for (path, source) in [
        (
            "server/errors/OrderNotFound.js",
            "class OrderNotFound extends Error {}\nthrow new Error('side effect')",
        ),
        (
            "server/errors/OrderNotFound.js",
            "class OrderNotFound extends Error {}\nclass Other extends Error {}",
        ),
        (
            "server/errors/OrderNotFound.js",
            "@Service\nclass OrderNotFound {}",
        ),
        (
            "server/errors/OrderNotFound.js",
            "class OrderNotFound extends makeError() {}",
        ),
        (
            "server/errors/OrderNotFound.js",
            "class OrderNotFound extends Error { static { run(); } }",
        ),
        (
            "server/errors/nested/OrderNotFound.js",
            "class OrderNotFound extends Error {}",
        ),
        (
            "client/errors/OrderNotFound.js",
            "class OrderNotFound extends Error {}",
        ),
        (
            "server/errors/OrderNotFound.swift",
            "class OrderNotFound {}",
        ),
        (
            "server/errors/OrderNotFound.rs",
            "struct OrderNotFound;\nimpl Other {}",
        ),
        (
            "server/models/OrderNotFound.go",
            "type OrderNotFound struct{}\nfunc (e *OrderNotFound) Error() string { return \"\" }",
        ),
    ] {
        assert!(
            validate(Path::new(path), source).is_err(),
            "{path}: {source}"
        );
    }
}
