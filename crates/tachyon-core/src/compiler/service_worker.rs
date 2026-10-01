use super::WebCompiler;
use super::content_digest::ContentDigest;
use super::portable_path::PortablePath;
use crate::Failure;
use crate::failure::diagnostic;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) struct ServiceWorker;

impl ServiceWorker {
    const MAX_ASSET_BYTES: usize = 4 * 1_048_576;

    pub(super) fn cache_rules_literal(
        cache_rules: &[crate::native::CacheRule],
    ) -> Result<String, Failure> {
        serde_json::to_string(cache_rules).map_err(|error| {
            Failure::one(diagnostic(
                1502,
                format!("Cannot serialise the declared cache rules: {error}"),
                Some(String::from("Report this as a Tachyon bug.")),
                None,
            ))
        })
    }

    pub(super) fn version(
        files: &[(PathBuf, Vec<u8>)],
        build_config_digest: Option<&str>,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(WebCompiler::SERVICE_WORKER.as_bytes());
        hasher.update(WebCompiler::SERVICE_WORKER_REGISTRATION.as_bytes());
        Self::hash_config(&mut hasher, build_config_digest);
        Self::hash_files(&mut hasher, files);
        let digest = ContentDigest::hex(hasher.finalize());
        digest.get(..16).unwrap_or("0").to_owned()
    }

    pub(super) fn assets_with_profile(
        files: &[(PathBuf, Vec<u8>)],
        profile: Option<crate::tac_python::RuntimeProfile>,
    ) -> Result<String, Failure> {
        let mut assets = BTreeMap::new();
        Self::add_assets(files, &mut assets, profile)?;
        serde_json::to_string(&assets).map_err(|_| {
            Failure::one(diagnostic(
                1201,
                "Cannot encode public asset fingerprints.",
                None,
                None,
            ))
        })
    }

    fn hash_config(hasher: &mut Sha256, build_config_digest: Option<&str>) {
        if let Some(digest) = build_config_digest {
            hasher.update(b"client/manifest.json\0");
            hasher.update(digest.as_bytes());
            hasher.update([0]);
        }
    }

    fn hash_files(hasher: &mut Sha256, files: &[(PathBuf, Vec<u8>)]) {
        let mut ordered: Vec<(String, &Vec<u8>)> = files
            .iter()
            .map(|(path, bytes)| (PortablePath::of(path), bytes))
            .collect();
        ordered.sort_by(|left, right| left.0.cmp(&right.0));
        for (path, bytes) in ordered {
            hasher.update(path.as_bytes());
            hasher.update([0]);
            hasher.update(bytes);
        }
    }

    fn add_assets(
        files: &[(PathBuf, Vec<u8>)],
        assets: &mut BTreeMap<String, Value>,
        profile: Option<crate::tac_python::RuntimeProfile>,
    ) -> Result<(), Failure> {
        for (path, bytes) in files {
            Self::add_profile_asset(path, bytes, assets, profile)?;
        }
        Ok(())
    }

    fn add_profile_asset(
        path: &Path,
        bytes: &[u8],
        assets: &mut BTreeMap<String, Value>,
        profile: Option<crate::tac_python::RuntimeProfile>,
    ) -> Result<(), Failure> {
        let portable = PortablePath::of(path);
        let ceiling = profile
            .map(|profile| profile.asset_limit(&portable, bytes))
            .transpose()?
            .unwrap_or(Self::MAX_ASSET_BYTES);
        Self::add_bounded_asset(&portable, bytes, ceiling, assets);
        Self::add_ceiling(&portable, ceiling, assets);
        Ok(())
    }

    fn add_bounded_asset(
        portable: &str,
        bytes: &[u8],
        ceiling: usize,
        assets: &mut BTreeMap<String, Value>,
    ) {
        if bytes.len() <= ceiling {
            Self::add_asset(portable, bytes, assets);
        }
    }

    fn add_ceiling(portable: &str, ceiling: usize, assets: &mut BTreeMap<String, Value>) {
        if ceiling > Self::MAX_ASSET_BYTES {
            assets.entry(format!("/{portable}")).and_modify(|asset| {
                asset["bodyLimit"] = serde_json::json!(ceiling);
            });
        }
    }

    fn add_asset(portable: &str, bytes: &[u8], assets: &mut BTreeMap<String, Value>) {
        let path = format!("/{portable}");
        let asset = serde_json::json!({ "sha256": ContentDigest::sha256_bytes(bytes), "bytes": bytes.len() });
        assets.insert(path.clone(), asset.clone());
        assets.extend(Self::directory_aliases(&path, &asset));
    }

    fn directory_aliases(path: &str, asset: &Value) -> Vec<(String, Value)> {
        path.strip_suffix("index.html")
            .map(|directory| Self::aliases_for(directory, asset))
            .unwrap_or_default()
    }

    fn aliases_for(directory: &str, asset: &Value) -> Vec<(String, Value)> {
        let mut aliases = vec![(directory.to_owned(), asset.clone())];
        if directory != "/" {
            aliases.push((directory.trim_end_matches('/').to_owned(), asset.clone()));
        }
        aliases
    }
}

#[cfg(test)]
mod tests {
    use super::ServiceWorker;
    use crate::tac_python::RuntimeProfile;
    use std::path::{Path, PathBuf};

    #[test]
    fn ordinary_assets_keep_exact_metadata_and_the_four_mib_ceiling() {
        let files = vec![
            (PathBuf::from("ordinary.js"), vec![1]),
            (PathBuf::from("big.wasm"), vec![0; 4 * 1024 * 1024 + 1]),
        ];
        let metadata: serde_json::Value = serde_json::from_str(
            &ServiceWorker::assets_with_profile(&files, None)
                .unwrap_or_else(|error| panic!("ordinary assets: {error:?}")),
        )
        .unwrap_or_else(|error| panic!("JSON: {error:?}"));
        assert!(metadata["/ordinary.js"]["bodyLimit"].is_null());
        assert_eq!(
            metadata["/ordinary.js"]
                .as_object()
                .unwrap_or_else(|| panic!("asset"))
                .len(),
            2
        );
        assert!(metadata["/big.wasm"].is_null());
    }

    #[test]
    fn a_profile_refuses_tampered_pinned_wasm_and_never_relaxes_other_files() {
        let profile = RuntimeProfile::read(&serde_json::json!({"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}})).unwrap_or_else(|error| panic!("profile: {error:?}"));
        let tampered = vec![(
            PathBuf::from(".tachyon/services/python/314.0.7/pyodide.asm.wasm"),
            vec![0],
        )];
        assert!(ServiceWorker::assets_with_profile(&tampered, profile).is_err());
        let unrelated = vec![(PathBuf::from("other.wasm"), vec![0; 4 * 1024 * 1024 + 1])];
        assert_eq!(
            ServiceWorker::assets_with_profile(&unrelated, profile)
                .unwrap_or_else(|error| panic!("unchanged default: {error:?}")),
            "{}"
        );
    }

    #[test]
    #[ignore = "requires pinned core Wasm; set TAC_PYTHON_CORE to the verified local directory"]
    fn only_verified_pinned_wasm_receives_the_explicit_decoded_body_ceiling() {
        let root = std::env::var_os("TAC_PYTHON_CORE")
            .unwrap_or_else(|| panic!("explicit runtime assets"));
        let bytes = std::fs::read(Path::new(&root).join("pyodide.asm.wasm"))
            .unwrap_or_else(|error| panic!("pinned Wasm: {error:?}"));
        let files = vec![(
            PathBuf::from(".tachyon/services/python/314.0.7/pyodide.asm.wasm"),
            bytes,
        )];
        let profile = RuntimeProfile::read(&serde_json::json!({"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}})).unwrap_or_else(|error| panic!("profile: {error:?}"));
        let metadata: serde_json::Value = serde_json::from_str(
            &ServiceWorker::assets_with_profile(&files, profile)
                .unwrap_or_else(|error| panic!("verified runtime metadata: {error:?}")),
        )
        .unwrap_or_else(|error| panic!("JSON: {error:?}"));
        assert_eq!(
            metadata["/.tachyon/services/python/314.0.7/pyodide.asm.wasm"]["bodyLimit"],
            10 * 1024 * 1024
        );
    }
}
