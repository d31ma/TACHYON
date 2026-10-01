use super::ClassFileName;
use std::path::Path;

#[test]
fn rust_snake_case_names_preserve_acronyms_digits_and_legacy_names() {
    for (name, snake) in [
        ("RouteState", "route_state"),
        ("HTTPClient", "http_client"),
        ("URLParserService", "url_parser_service"),
        ("Html5Parser", "html5_parser"),
        ("HTTP2Service", "http2_service"),
    ] {
        let path = format!("client/services/nested/{snake}.rs");
        assert!(ClassFileName::matches(Path::new(&path), name), "{name}");
        assert_eq!(ClassFileName::preferred(Path::new(&path), name), snake);
        let legacy = format!("client/services/{name}.rs");
        assert!(ClassFileName::matches(Path::new(&legacy), name));
    }
    for stem in [
        "route__state",
        "Route_State",
        "route_State",
        "routes_state",
        "_route_state",
        "route_state_",
    ] {
        assert!(!ClassFileName::matches(
            Path::new(&format!("{stem}.rs")),
            "RouteState"
        ));
    }
    assert!(!ClassFileName::matches(
        Path::new("route_state.ts"),
        "RouteState"
    ));
}

#[test]
fn rust_snake_case_identity_comes_from_the_actual_type() {
    let path = Path::new("client/services/http_reader_service.rs");
    let source =
        "#[Service] #[derive(Default)] struct HTTPReaderService; impl HTTPReaderService {}";
    assert_eq!(
        ClassFileName::declared(path, source).as_deref(),
        Some("HTTPReaderService")
    );
}
