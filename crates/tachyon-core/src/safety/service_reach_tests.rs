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
