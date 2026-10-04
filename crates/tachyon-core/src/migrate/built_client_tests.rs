use super::built_client::BuiltClient;

#[test]
fn a_service_that_builds_its_client_is_reported_in_every_form() {
    let built = [
        ("client/services/A.rs", "catalog: CatalogClient::new(),"),
        ("client/services/A.rs", "catalog: CatalogClient::with_context(context),"),
        ("client/services/A.java", "CatalogClient catalog = new CatalogClient();"),
        ("client/services/A.cs", "private readonly CatalogClient _catalog = new CatalogClient();"),
        ("client/services/A.kt", "private val catalog = CatalogClient()"),
        ("client/services/A.swift", "let catalog = CatalogClient()"),
        ("client/services/A.go", "catalog, err := NewCatalogClient()"),
        ("client/services/A.rb", "@catalog = CatalogClient.new"),
        ("client/services/A.dart", "final catalog = CatalogClient();"),
    ];
    for (path, source) in built {
        assert!(BuiltClient::finding(path, source).is_some(), "{path}: {source}");
    }
}

#[test]
fn an_injected_client_is_not_reported() {
    let kept = [
        ("client/services/A.rs", "#[Needs]\ncatalog_client: std::sync::Arc<CatalogClient>,"),
        ("client/services/A.java", "@Needs\nprivate final CatalogClient catalogClient;"),
        ("client/services/A.kt", "@Needs\nprivate val catalogClient: CatalogClient"),
        ("client/services/A.rs", "// CatalogClient::new() is built by Tachyon"),
    ];
    for (path, source) in kept {
        assert!(BuiltClient::finding(path, source).is_none(), "{path}: {source}");
    }
}
