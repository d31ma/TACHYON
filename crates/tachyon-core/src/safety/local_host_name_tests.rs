use super::{DeclaredHosts, LocalHostName};

#[test]
fn localhost_and_names_under_it_are_local() {
    for name in [
        "localhost",
        "LOCALHOST",
        "localhost.",
        "app.localhost",
        "app.localhost.",
        "a.b.LocalHost",
        ".localhost",
    ] {
        assert!(LocalHostName::is_local(name), "{name}");
    }
    for name in [
        "localhost..",
        "notlocalhost",
        "localhost2",
        "localhost.example",
        "127.0.0.1",
        "::1",
        "",
    ] {
        assert!(!LocalHostName::is_local(name), "{name}");
    }
}

#[test]
fn a_bind_name_is_a_valid_local_name_in_lower_case_without_the_trailing_dot() {
    let accepted = [
        ("localhost", "localhost"),
        ("LOCALHOST", "localhost"),
        ("localhost.", "localhost"),
        ("Shop.localhost", "shop.localhost"),
        ("app.localhost.", "app.localhost"),
        ("a.b.LocalHost", "a.b.localhost"),
        ("my-app.localhost", "my-app.localhost"),
    ];
    for (text, name) in accepted {
        assert_eq!(
            LocalHostName::bind_name(text).as_deref(),
            Some(name),
            "{text}"
        );
    }
    let long_label = format!("{}.localhost", "a".repeat(64));
    let refused = [
        ".localhost",
        "localhost..",
        "a..localhost",
        "-app.localhost",
        "app_1.localhost",
        "notlocalhost",
        "localhost2",
        "localhost.example",
        "127.0.0.1",
        "::1",
        "",
        long_label.as_str(),
    ];
    for text in refused {
        assert_eq!(LocalHostName::bind_name(text), None, "{text}");
    }
}

#[test]
fn server_host_entries_are_names_or_subdomain_patterns() {
    for entry in [
        "app.example.com",
        "*.example.com",
        "intranet",
        "my-app.test",
    ] {
        assert!(DeclaredHosts::is_name_entry(entry), "{entry}");
    }
    for entry in [
        "*",
        "",
        "*.",
        "https://app.example.com",
        "app.example.com:8080",
        "app.example.com/path",
        "a..example.com",
        "app.*.example.com",
        "[::1]",
    ] {
        assert!(!DeclaredHosts::is_name_entry(entry), "{entry}");
    }
}
