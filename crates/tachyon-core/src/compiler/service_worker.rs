use super::WebCompiler;
use super::content_digest::ContentDigest;
use super::portable_path::PortablePath;
use crate::Failure;
use crate::failure::diagnostic;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;

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

    pub(super) fn assets(files: &[(PathBuf, Vec<u8>)]) -> Result<String, Failure> {
        let mut assets = BTreeMap::new();
        Self::add_assets(files, &mut assets);
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

    fn add_assets(files: &[(PathBuf, Vec<u8>)], assets: &mut BTreeMap<String, Value>) {
        let small = files
            .iter()
            .filter(|(_, bytes)| bytes.len() <= Self::MAX_ASSET_BYTES);
        for (path, bytes) in small {
            Self::add_asset(&PortablePath::of(path), bytes, assets);
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
