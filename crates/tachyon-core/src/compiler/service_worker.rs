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
    const MAX_SERVICE_ASSET_BYTES: usize = 16 * 1_048_576;
    const CEILINGS: [usize; 2] = [Self::MAX_ASSET_BYTES, Self::MAX_SERVICE_ASSET_BYTES];
    const SERVICE_FILES: &'static str = ".tachyon/services/";
    const PYTHON_RUNTIME: &'static str = ".tachyon/services/python/";

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

    pub(super) fn offline_notes(
        files: &[(PathBuf, Vec<u8>)],
        profile: Option<crate::tac_python::RuntimeProfile>,
    ) -> Result<Vec<String>, Failure> {
        let notes = files
            .iter()
            .map(|(path, bytes)| Self::offline_note(path, bytes, profile))
            .collect::<Result<Vec<Option<String>>, Failure>>()?;
        Ok(notes.into_iter().flatten().collect())
    }

    fn offline_note(
        path: &Path,
        bytes: &[u8],
        profile: Option<crate::tac_python::RuntimeProfile>,
    ) -> Result<Option<String>, Failure> {
        let portable = PortablePath::of(path);
        let ceiling = Self::ceiling(&portable, bytes, profile)?;
        Ok((bytes.len() > ceiling).then(|| {
            format!(
                "Offline: /{portable} is {} KiB, over the service worker's {} KiB limit for one file, so the worker does not keep it and a page that loads it needs the network. Make the file smaller, or split the service that produces it.",
                bytes.len() / 1024,
                ceiling / 1024
            )
        }))
    }

    fn ceiling(
        portable: &str,
        bytes: &[u8],
        profile: Option<crate::tac_python::RuntimeProfile>,
    ) -> Result<usize, Failure> {
        let python = profile
            .map(|_| crate::tac_python::RuntimeProfile::asset_limit(portable, bytes))
            .transpose()?;
        let service = portable.starts_with(Self::SERVICE_FILES)
            && !portable.starts_with(Self::PYTHON_RUNTIME);
        Ok(python
            .filter(|limit| *limit > Self::MAX_ASSET_BYTES)
            .unwrap_or(Self::CEILINGS[usize::from(service)]))
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
        let ceiling = Self::ceiling(&portable, bytes, profile)?;
        Self::add_bounded_asset(&portable, bytes, ceiling, assets);
        Self::add_ceiling(&portable, bytes.len(), ceiling, assets);
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

    fn add_ceiling(
        portable: &str,
        length: usize,
        ceiling: usize,
        assets: &mut BTreeMap<String, Value>,
    ) {
        if length > Self::MAX_ASSET_BYTES {
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
    fn a_file_the_worker_cannot_keep_is_named_in_a_build_note() {
        let files = vec![
            (PathBuf::from("ordinary.js"), vec![1]),
            (PathBuf::from("at-limit.wasm"), vec![0; 4 * 1024 * 1024]),
            (PathBuf::from("shared/video.webm"), vec![0; 5 * 1024 * 1024]),
            (
                PathBuf::from(".tachyon/services/go/services.wasm"),
                vec![0; 5 * 1024 * 1024],
            ),
        ];
        let notes = ServiceWorker::offline_notes(&files, None)
            .unwrap_or_else(|error| panic!("notes: {error:?}"));
        assert_eq!(
            notes,
            vec![String::from(
                "Offline: /shared/video.webm is 5120 KiB, over the service worker's 4096 KiB limit for one file, so the worker does not keep it and a page that loads it needs the network. Make the file smaller, or split the service that produces it."
            )]
        );
    }

    #[test]
    fn service_files_may_reach_sixteen_mib_except_the_python_runtime() {
        let go = vec![0; 5 * 1024 * 1024];
        let files = vec![
            (PathBuf::from(".tachyon/services/go/services.wasm"), go),
            (PathBuf::from(".tachyon/services/Small.js"), vec![1]),
            (
                PathBuf::from(".tachyon/services/Huge.wasm"),
                vec![0; 16 * 1024 * 1024 + 1],
            ),
            (
                PathBuf::from(".tachyon/services/python/314.0.7/pyodide.asm.wasm"),
                vec![0; 5 * 1024 * 1024],
            ),
        ];
        let metadata: serde_json::Value = serde_json::from_str(
            &ServiceWorker::assets_with_profile(&files, None)
                .unwrap_or_else(|error| panic!("service assets: {error:?}")),
        )
        .unwrap_or_else(|error| panic!("JSON: {error:?}"));
        assert_eq!(
            metadata["/.tachyon/services/go/services.wasm"]["bodyLimit"],
            16 * 1024 * 1024
        );
        assert!(metadata["/.tachyon/services/Small.js"]["bodyLimit"].is_null());
        assert!(metadata["/.tachyon/services/Huge.wasm"].is_null());
        assert!(metadata["/.tachyon/services/python/314.0.7/pyodide.asm.wasm"].is_null());
        let notes = ServiceWorker::offline_notes(&files, None)
            .unwrap_or_else(|error| panic!("notes: {error:?}"));
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(
            notes[0].contains("Huge.wasm is 16384 KiB, over the service worker's 16384 KiB limit")
        );
        assert!(
            notes[1]
                .contains("pyodide.asm.wasm is 5120 KiB, over the service worker's 4096 KiB limit")
        );
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
