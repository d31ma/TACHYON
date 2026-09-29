use super::{Input, Marker, Type};
use crate::Failure;
use std::path::Path;

pub(crate) struct NativeStorage;

struct NeededAdapter {
    adapter: &'static str,
    uses: &'static str,
    name: String,
    owner: String,
    at: usize,
}

impl NativeStorage {
    const CACHE_METHODS: [&'static str; 3] = ["cacheable", "cacheEvict", "cachePut"];
    const BUILT_IN: &'static str = "The built-in browser cache and Store work for JavaScript, TypeScript, Dart and Ruby services and for Go, Rust, C#, Kotlin and Swift WebAssembly services";

    pub(crate) fn refuse_builtins(path: &Path, source: &str) -> Result<(), Failure> {
        let Some(rewritten) = crate::behavior::HttpClientSource::rewrite(path, source)? else {
            return Self::refuse_in(path, source);
        };
        let roles = crate::behavior::ServiceRoles::read(path, &rewritten)?;
        Self::refuse_in(path, &roles.source)
    }

    fn refuse_in(path: &Path, source: &str) -> Result<(), Failure> {
        let language = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let mask = crate::lexical::code_mask(language, source);
        let mut input = Input {
            path,
            source,
            code: super::visible_code(source, mask.code),
        };
        super::check_behavior_source(&input, mask.error, "tac")?;
        let markers: Vec<Marker> = super::annotations(&input, language, "tac")?
            .into_iter()
            .filter(|marker| {
                marker.name != "Schedule" && !crate::behavior::StoredState::marks(&marker.name)
            })
            .collect();
        if markers.is_empty() {
            return Ok(());
        }
        super::mask_annotations(&mut input, &markers, language);
        let types = super::behavior_types(&input, language, "tac", &markers)?;
        let setups = Self::setup_texts(&input, language, &types);
        for needed in Self::needed_adapters(&types) {
            let quoted = format!("\"{}\"", needed.name);
            if !setups.iter().any(|setup| setup.contains(&quoted)) {
                return Err(input.failure(needed.at, Self::refusal(language, &needed)));
            }
        }
        Ok(())
    }

    fn setup_texts<'a>(input: &Input<'a>, language: &str, types: &[Type]) -> Vec<&'a str> {
        let mut texts = Vec::new();
        for setup in types.iter().filter(|ty| ty.configuration) {
            let body = if language == "rs" {
                super::rust_impl_body(input, &setup.name).unwrap_or_else(|| setup.body.clone())
            } else {
                setup.body.clone()
            };
            texts.push(&input.source[body]);
        }
        texts
    }

    fn needed_adapters(types: &[Type]) -> Vec<NeededAdapter> {
        let mut needed = Vec::new();
        for ty in types {
            let caches = ty.methods.iter().any(|method| {
                Self::CACHE_METHODS
                    .iter()
                    .any(|key| method.metadata.contains_key(*key))
            });
            let cache = ty.cache.as_ref().and_then(|cache| cache["name"].as_str());
            if let Some(name) = cache
                && caches
            {
                needed.push(NeededAdapter {
                    adapter: "a cache adapter",
                    uses: "the cache",
                    name: name.to_owned(),
                    owner: ty.name.clone(),
                    at: ty.declaration,
                });
            }
            let entity = ty
                .repository
                .as_ref()
                .and_then(|store| store["entity"].as_str());
            let adapter = types
                .iter()
                .find(|other| Some(other.name.as_str()) == entity)
                .and_then(|other| other.entity.as_ref())
                .and_then(|options| options["adapter"].as_str());
            if let Some(name) = adapter {
                needed.push(NeededAdapter {
                    adapter: "an entity adapter",
                    uses: "the entity adapter",
                    name: name.to_owned(),
                    owner: ty.name.clone(),
                    at: ty.declaration,
                });
            }
        }
        needed
    }

    fn refusal(language: &str, needed: &NeededAdapter) -> String {
        let services = match language {
            "kt" => "Android Kotlin services",
            "swift" => "Swift services",
            "rs" => "Native Rust services",
            "cs" => "Native C# services",
            _ => "Native services",
        };
        format!(
            "{services} need {adapter} in their Setup: {owner} uses {uses} \"{name}\", and no Setup names it. {built_in}; {services} answer the page from another process, so they cannot reach them.",
            adapter = needed.adapter,
            owner = needed.owner,
            uses = needed.uses,
            name = needed.name,
            built_in = Self::BUILT_IN,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::NativeStorage;
    use std::path::Path;

    const KOTLIN_SERVICE: &str = "@CacheSettings({\"name\":\"catalog\",\"ttlMs\":30000,\"maxBytes\":65536,\"maxEntries\":256})\nclass CatalogService {\n    @Cached\n    fun find(sku: String): Any? {\n        return mapOf(\"sku\" to sku)\n    }\n}\n";
    const KOTLIN_SETUP: &str = "@Setup\nclass CatalogSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration(\n            caches = mapOf(\"catalog\" to MemoryCache()))\n    }\n}\n";
    const KOTLIN_EMPTY_SETUP: &str = "@Setup\nclass CatalogSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n";
    const SWIFT_STORE: &str = "@Entity({\"adapter\":\"publicCatalog\"})\n@Table(\"products\")\nfinal class ProductEntity {\n    @Id\n    var id: Int = 0\n}\n@Store(ProductEntity, {\"fields\":[\"id\",\"status\"],\"defaultSize\":20,\"maxSize\":100})\nfinal class ProductStore {\n    @Needs\n    let behaviors: BehaviorContext\n    func findByStatus(_ status: BehaviorJSON) async throws -> BehaviorJSON {}\n}\n";
    const RUST_SERVICE: &str = "#[CacheSettings({\"name\":\"catalog\",\"ttlMs\":30000,\"maxBytes\":65536,\"maxEntries\":256})]\nstruct CatalogService {}\nimpl CatalogService {\n    #[Cached]\n    fn find(&self, sku: BehaviorValue) -> BehaviorResult<BehaviorValue> {\n        Ok(sku)\n    }\n}\n";
    const RUST_SETUP: &str = "#[tac::setup]\nstruct CatalogSetup {}\nimpl CatalogSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        let mut configuration = BehaviorConfiguration::default();\n        configuration.caches.insert(\"catalog\".into(), std::sync::Arc::new(MemoryCache::default()));\n        Ok(configuration)\n    }\n}\n";

    fn refusal(path: &str, source: &str) -> String {
        NativeStorage::refuse_builtins(Path::new(path), source)
            .expect_err("refused")
            .to_string()
    }

    #[test]
    fn a_kotlin_cache_without_a_setup_adapter_is_refused() {
        let message = refusal("client/services/CatalogService.kt", KOTLIN_SERVICE);
        assert!(
            message.contains("Kotlin services need a cache adapter in their Setup"),
            "{message}"
        );
        assert!(message.contains("\"catalog\""), "{message}");
        assert!(
            message.contains("Go, Rust, C#, Kotlin and Swift WebAssembly"),
            "{message}"
        );
        let empty = format!("{KOTLIN_SERVICE}{KOTLIN_EMPTY_SETUP}");
        assert!(refusal("client/services/CatalogService.kt", &empty).contains("Kotlin services"));
    }

    #[test]
    fn a_kotlin_cache_named_by_the_setup_is_accepted() {
        let source = format!("{KOTLIN_SERVICE}{KOTLIN_SETUP}");
        NativeStorage::refuse_builtins(Path::new("client/services/CatalogService.kt"), &source)
            .expect("the Setup names the cache");
    }

    #[test]
    fn a_swift_store_without_an_entity_adapter_is_refused() {
        let message = refusal("client/services/ProductService.swift", SWIFT_STORE);
        assert!(
            message.contains("Swift services need an entity adapter in their Setup"),
            "{message}"
        );
        assert!(message.contains("\"publicCatalog\""), "{message}");
    }

    #[test]
    fn a_native_rust_cache_needs_its_setup_adapter() {
        let message = refusal("client/services/CatalogService.rs", RUST_SERVICE);
        assert!(
            message.contains("Native Rust services need a cache adapter in their Setup"),
            "{message}"
        );
        let source = format!("{RUST_SERVICE}{RUST_SETUP}");
        NativeStorage::refuse_builtins(Path::new("client/services/CatalogService.rs"), &source)
            .expect("the namespaced Setup names the cache");
    }

    #[test]
    fn a_service_without_storage_markers_is_accepted() {
        NativeStorage::refuse_builtins(
            Path::new("client/services/PlainService.kt"),
            "class PlainService {\n    var count: Int = 0\n}\n",
        )
        .expect("plain");
    }
}
