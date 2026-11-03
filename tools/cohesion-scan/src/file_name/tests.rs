use super::FileName;
use crate::structure::Structure;

fn structure(types: &[&str]) -> Structure {
    Structure {
        types: types.iter().map(ToString::to_string).collect(),
        ..Structure::default()
    }
}

#[test]
fn type_names_become_snake_case() {
    assert_eq!(FileName::snake_case("RouteState"), "route_state");
    assert_eq!(FileName::snake_case("HTTPClient"), "http_client");
    assert_eq!(FileName::snake_case("Html5Parser"), "html5_parser");
    assert_eq!(FileName::snake_case("Scan"), "scan");
}

#[test]
fn a_file_named_after_its_type_passes() {
    assert_eq!(
        FileName::wanted("src/route_state.rs", &structure(&["RouteState"])),
        None
    );
    assert_eq!(FileName::wanted("src/mod.rs", &structure(&["Other"])), None);
    assert_eq!(FileName::wanted("src/lib.rs", &structure(&["Scan"])), None);
    assert_eq!(FileName::wanted("src/helpers.rs", &structure(&[])), None);
}

#[test]
fn a_file_named_after_something_else_names_the_wanted_file() {
    let wanted = FileName::wanted(
        "src/browser_interop.rs",
        &structure(&["BrowserInteropCheck"]),
    );
    assert_eq!(wanted.as_deref(), Some("browser_interop_check.rs"));
}
