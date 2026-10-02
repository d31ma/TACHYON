use super::DeclaredContract;
use std::path::Path;

struct CsharpTacContextSamples;

impl CsharpTacContextSamples {
    const CLIENT: &'static str = "[HTTPClient({\"setup\":\"CustomSetup\",\"adapter\":\"catalog\"})]\npublic interface CatalogClient {\n [GetRequest(\"/products\")]\n System.Threading.Tasks.Task<BehaviorJSON> all();\n}\n";

    fn lower(source: &str, environment: &str) -> Result<String, String> {
        DeclaredContract::concrete(
            Path::new("client/remotes/CatalogClient.cs"),
            source,
            environment,
        )
        .ok_or_else(|| "concrete C# contract".to_owned())
    }
}

#[test]
fn csharp_tac_http_context_preserves_default_and_captured_construction() -> Result<(), String> {
    let source = CsharpTacContextSamples::lower(CsharpTacContextSamples::CLIENT, "tac")?;
    assert!(source.contains("private readonly BehaviorContext behaviors;"));
    assert!(source.contains("public CatalogClient() : this(Behaviors.create(typeof(CustomSetup), \"catalog\", new BehaviorSignal())) {}"));
    assert!(source.contains("public CatalogClient(BehaviorContext context)"));
    assert!(source.contains(
        "behaviors = context ?? throw new System.ArgumentNullException(nameof(context));"
    ));
    assert_eq!(source.matches("Behaviors.create(").count(), 1);
    assert!(!source.contains("OnStart"));
    assert!(!source.contains("OnStop"));
    assert!(!source.contains("Dispose("));
    Ok(())
}

#[test]
fn csharp_tac_http_context_does_not_duplicate_authored_constructors() {
    let source = "[HTTPClient]\npublic class CatalogClient {\n [Needs]\n private readonly BehaviorContext behaviors;\n public CatalogClient(BehaviorContext context) { behaviors = context; }\n [GetRequest]\n public async System.Threading.Tasks.Task<BehaviorJSON> all() {}\n}\n";
    assert!(
        DeclaredContract::concrete(Path::new("client/remotes/CatalogClient.cs"), source, "tac")
            .is_none()
    );
}

#[test]
fn csharp_tac_http_context_keeps_yon_and_store_ownership_unchanged() -> Result<(), String> {
    let yon = CsharpTacContextSamples::lower(CsharpTacContextSamples::CLIENT, "yon")?;
    assert!(yon.contains("[OnStart]"));
    assert!(!yon.contains("CatalogClient(BehaviorContext context)"));
    let store = "[Store(typeof(ProductEntity))]\npublic interface ProductStore {\n System.Threading.Tasks.Task<BehaviorJSON> findById(string id);\n}\n";
    let store =
        DeclaredContract::concrete(Path::new("client/stores/ProductStore.cs"), store, "tac")
            .ok_or_else(|| "concrete C# Store".to_owned())?;
    assert!(
        store.contains("public BehaviorContext behaviors = Behaviors.create(typeof(ProductSetup)")
    );
    assert!(!store.contains("ProductStore(BehaviorContext context)"));
    Ok(())
}
