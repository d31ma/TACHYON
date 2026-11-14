use super::ServiceReachCheck;
use std::path::Path;

fn codes(path: &str, source: &str) -> Vec<String> {
    ServiceReachCheck::diagnostics([(Path::new(path), source.as_bytes())].into_iter())
        .iter()
        .map(|diagnostic| diagnostic.code.to_string())
        .collect()
}

#[test]
fn a_tac_service_that_reads_global_this_is_refused() {
    let source = "@Service\nexport default class ReportService {\n  minimize() {\n    globalThis.tachyonWindow?.minimize();\n  }\n}\n";
    assert_eq!(
        codes("client/services/ReportService.js", source),
        ["TY1360"]
    );
    assert_eq!(
        codes("client/services/ReportService.ts", source),
        ["TY1360"]
    );
}

#[test]
fn a_tac_service_that_calls_fetch_is_refused() {
    for call in [
        "fetch('/api/products')",
        "window.fetch('/api/products')",
        "await fetch ('/api')",
    ] {
        let source = format!(
            "@Service\nexport default class CatalogService {{\n  async load() {{\n    return {call};\n  }}\n}}\n"
        );
        assert_eq!(
            codes("client/services/CatalogService.js", &source),
            ["TY1361"],
            "{call}"
        );
    }
}

#[test]
fn injected_members_setups_strings_and_comments_are_not_refused() {
    let source = "@Service\nexport default class CatalogService {\n  // fetch('/api') and globalThis live in HTTPClient code\n  label = 'fetch(globalThis)';\n  async load() {\n    this.fetcher.fetch('/api');\n    return this.prefetch();\n  }\n}\n";
    assert!(codes("client/services/CatalogService.js", source).is_empty());
    let setup = "@Setup\nexport default class CatalogSetup {\n  static configure() {\n    return { http: { catalog: { exchange: () => fetch('/api/products') } } };\n  }\n}\n";
    assert!(codes("client/setups/CatalogSetup.js", setup).is_empty());
    assert!(
        codes(
            "server/services/CatalogService.js",
            "run() { return fetch('/x') }\n"
        )
        .is_empty()
    );
}

#[test]
fn the_fetch_refusal_names_the_service_the_file_and_the_line() {
    let source = "@Service\nexport default class CatalogService {\n  async load() {\n    return fetch('/api/products');\n  }\n}\n";
    let found = ServiceReachCheck::diagnostics(
        [(
            Path::new("client/services/CatalogService.ts"),
            source.as_bytes(),
        )]
        .into_iter(),
    );
    assert_eq!(
        found[0].message,
        "CatalogService calls the network itself with `fetch` in 'client/services/CatalogService.ts' line 4. Tac code reaches the network only through a remote: an HTTPClient in client/remotes."
    );
}

#[test]
fn a_tac_service_that_builds_an_xml_http_request_is_refused() {
    let source = "@Service\nexport default class CatalogService {\n  load() {\n    const request = new XMLHttpRequest();\n  }\n}\n";
    assert_eq!(
        codes("client/services/CatalogService.js", source),
        ["TY1361"]
    );
}

#[test]
fn a_tac_service_that_sends_a_beacon_is_refused() {
    let source = "@Service\nexport default class CatalogService {\n  leave() {\n    navigator.sendBeacon('/api/leave');\n  }\n}\n";
    assert_eq!(
        codes("client/services/CatalogService.ts", source),
        ["TY1361"]
    );
}

#[test]
fn a_remote_method_named_send_beacon_is_not_refused() {
    let source = "@Service\nexport default class CatalogService {\n  leave() {\n    this.analytics.sendBeacon();\n  }\n}\n";
    assert!(codes("client/services/CatalogService.ts", source).is_empty());
}
