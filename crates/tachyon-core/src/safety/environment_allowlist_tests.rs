use super::EnvironmentAllowlist;

fn refusal(entry: &str) -> String {
    EnvironmentAllowlist::parse(&[entry])
        .err()
        .unwrap_or_default()
}

#[test]
fn names_and_prefix_patterns_are_kept_apart() {
    let allowlist =
        EnvironmentAllowlist::parse(&["HOME", "HD_*", "AWS_*", "_PRIVATE_APP"]).unwrap_or_default();

    assert!(allowlist.names().contains("HOME"));
    assert!(allowlist.names().contains("_PRIVATE_APP"));
    assert!(allowlist.prefixes().contains("HD_"));
    assert!(allowlist.prefixes().contains("AWS_"));
    assert_eq!(allowlist.names().len() + allowlist.prefixes().len(), 4);
}

#[test]
fn an_empty_list_passes_nothing_more() {
    let allowlist = EnvironmentAllowlist::parse(&[]).unwrap_or_default();

    assert!(allowlist.names().is_empty());
    assert!(allowlist.prefixes().is_empty());
}

#[test]
fn a_lone_wildcard_is_refused_because_it_passes_everything() {
    let message = refusal("*");

    assert!(message.contains("every variable"), "{message}");
}

#[test]
fn malformed_entries_are_refused_by_name() {
    for entry in [
        "",
        "1BAD",
        "BAD-NAME",
        "BAD=VALUE",
        "*HD",
        "HD_*_X",
        "NAÏVE",
        "HD**",
    ] {
        let message = refusal(entry);

        assert!(
            message.contains("not a variable name"),
            "{entry}: {message}"
        );
        assert!(
            message.contains(&format!("\"{entry}\"")),
            "{entry}: {message}"
        );
    }
}

#[test]
fn ty_internal_names_and_patterns_that_cover_them_are_refused() {
    for entry in [
        "TAC_DEV",
        "TACHYON_HOME",
        "YON_PROFILE",
        "yon_resource_token",
        "HTTP_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "NO_PROXY",
        "no_proxy",
        "NODE_USE_ENV_PROXY",
        "TAC_*",
        "TACHYON_*",
        "YON_*",
        "YON_RESOURCE_*",
        "T*",
        "TA*",
        "Y*",
        "H*",
        "HTTP*",
        "N*",
        "NO_*",
        "A*",
        "NODE_*",
    ] {
        let message = refusal(entry);

        assert!(message.contains("ty's own variables"), "{entry}: {message}");
    }
}

#[test]
fn prefixes_beside_the_reserved_ones_are_allowed() {
    let allowlist =
        EnvironmentAllowlist::parse(&["TACO_*", "HD_*", "HTTP_PROXY_EXTRA", "NODE_ENV"])
            .unwrap_or_default();

    assert!(allowlist.prefixes().contains("TACO_"));
    assert!(allowlist.names().contains("NODE_ENV"));
    assert!(allowlist.names().contains("HTTP_PROXY_EXTRA"));
}
