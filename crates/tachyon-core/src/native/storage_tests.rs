#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;

struct Sources;

impl Sources {
    const KOTLIN_SERVICE: &str = "@Service\n@CacheSettings({\"name\":\"catalog\",\"ttlMs\":30000,\"maxBytes\":65536,\"maxEntries\":256})\nclass CatalogService {\n    var reads: Int = 0\n\n    @Cached\n    fun find(sku: String): Any? {\n        reads++\n        return mapOf(\"sku\" to sku, \"reads\" to reads)\n    }\n}\n";
    const KOTLIN_SETUP: &str = "@Setup\nclass CatalogSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration(\n            authorize = Behaviors.Authorization { _, _ -> true })\n    }\n}\n";

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }
}

#[test]
fn a_kotlin_cache_without_a_setup_adapter_stops_the_native_build() {
    let root = tempfile::tempdir().expect("project");
    Sources::write(
        root.path(),
        "client/services/CatalogService.kt",
        Sources::KOTLIN_SERVICE,
    );
    Sources::write(
        root.path(),
        "client/setups/CatalogSetup.kt",
        Sources::KOTLIN_SETUP,
    );
    let inputs = [NativeCompanionInput {
        snapshot_root: root.path().to_path_buf(),
        language: NativeCompanion::Kotlin,
        source: root.path().join("client/services/CatalogService.kt"),
        service: String::from("catalog"),
        class: String::from("CatalogService"),
        web_assembly: false,
        application_provider: false,
    }];
    let failure = super::registry::source(&inputs, NativeCompanion::Kotlin)
        .expect_err("a Kotlin cache needs its Setup adapter");
    let message = failure.to_string();
    assert!(
        message.contains("Kotlin services need a cache adapter in their Setup"),
        "{message}"
    );
}
