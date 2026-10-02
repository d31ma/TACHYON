use super::DeclaredContract;
use std::path::Path;

struct DartTacContextSamples;

impl DartTacContextSamples {
    const CLIENT: &'static str = "@HTTPClient\nabstract interface class CatalogClient {\n @GetRequest(\"/products\")\n Future<Object?> all();\n}\n";

    fn lower(source: &str, environment: &str) -> String {
        DeclaredContract::concrete(
            Path::new("client/remotes/CatalogClient.dart"),
            source,
            environment,
        )
        .expect("concrete Dart contract")
    }
}

#[test]
fn dart_tac_http_context_is_captured_once_with_an_optional_constructor() {
    let source = DartTacContextSamples::lower(DartTacContextSamples::CLIENT, "tac");
    assert!(
        source.contains("final BehaviorContext? _suppliedContext;"),
        "{source}"
    );
    assert!(
        source.contains("CatalogClient([this._suppliedContext]);"),
        "{source}"
    );
    assert!(source.contains("late final BehaviorContext behaviors = _suppliedContext ?? Behaviors.create(CatalogSetup, partition: 'catalog', signal: BehaviorSignal());"), "{source}");
    assert!(!source.contains("OnStart"), "{source}");
    assert!(!source.contains("OnStop"), "{source}");
    assert!(!source.contains("dispose("), "{source}");
}

#[test]
fn dart_tac_http_context_keeps_named_default_setup_and_partition() {
    let source = DartTacContextSamples::CLIENT.replace(
        "@HTTPClient",
        "@HTTPClient({\"adapter\":\"api\",\"setup\":\"CustomSetup\"})",
    );
    let source = DartTacContextSamples::lower(&source, "tac");
    assert!(
        source.contains("CatalogClient([this._suppliedContext]);"),
        "{source}"
    );
    assert!(source.contains("_suppliedContext ?? Behaviors.create(CustomSetup, partition: 'api', signal: BehaviorSignal())"), "{source}");
}

#[test]
fn dart_tac_http_context_does_not_change_yon_context_lifecycle() {
    let source = DartTacContextSamples::lower(DartTacContextSamples::CLIENT, "yon");
    assert!(
        source.contains("late BehaviorContext behaviors;"),
        "{source}"
    );
    assert!(source.contains("@OnStart"), "{source}");
    assert!(source.contains("void startContext()"), "{source}");
    assert!(!source.contains("_suppliedContext"), "{source}");
}

#[test]
fn dart_tac_http_context_does_not_change_store_construction() {
    let source = "@Store(ProductEntity)\nabstract interface class ProductStore {\n Future<Object?> findById(String id);\n}\n";
    let source =
        DeclaredContract::concrete(Path::new("client/stores/ProductStore.dart"), source, "tac")
            .expect("concrete Dart Store");
    assert!(
        source.contains("late final BehaviorContext behaviors = Behaviors.create(ProductSetup"),
        "{source}"
    );
    assert!(!source.contains("_suppliedContext"), "{source}");
}
