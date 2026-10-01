use super::runtime_asset::RuntimeAsset;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use sha2::{Digest, Sha256};
use std::io::Read as _;
use std::path::{Path, PathBuf};

pub(crate) struct RuntimeAssets;

impl RuntimeAssets {
    pub(crate) const VERSION: &'static str = "314.0.7";
    pub(crate) const ABI: &'static str = "2026_0";
    pub(crate) const RUNTIME: &'static str = "pyodide-314.0.7";
    pub(crate) const OUTPUT: &'static str = ".tachyon/services/python/314.0.7";
    pub(crate) const WASM: &'static str = "pyodide.asm.wasm";
    const ASSETS: [RuntimeAsset; 5] = [
        RuntimeAsset {
            name: "pyodide.mjs",
            bytes: 17_931,
            sha256: "6f1d60f7bf529beb300f0f47983c921d3982363640ba20af0e38efdddbc66109",
        },
        RuntimeAsset {
            name: "pyodide.asm.mjs",
            bytes: 1_250_344,
            sha256: "f7cdc8ece80678ceb712f8e65ebe6d3a83203a180c399865f49612a051693635",
        },
        RuntimeAsset {
            name: "pyodide.asm.wasm",
            bytes: 9_598_218,
            sha256: "cc36e3cab04fdfc9a63ff13eb52eae2b911bf46c025cc7b281f394bd3de1d5e6",
        },
        RuntimeAsset {
            name: "python_stdlib.zip",
            bytes: 2_545_637,
            sha256: "fa1957e5777068fc4f7437f96d860ae2fbe9c19732ba06c84e004ec16dd7dd7a",
        },
        RuntimeAsset {
            name: "pyodide-lock.json",
            bytes: 119_077,
            sha256: "5dc2fc119108bc148c7457dc86e7675b5c87e1cafd420b9c34c1eaef7b36c010",
        },
    ];

    pub(crate) fn cached_root() -> PathBuf {
        crate::cache::root().join("python-web").join(Self::VERSION)
    }

    pub(crate) fn read(root: &Path) -> Result<Vec<(String, Vec<u8>)>, Failure> {
        Self::read_assets(root)
    }

    pub(crate) fn names() -> Vec<String> {
        Self::ASSETS.iter().map(Self::output_path).collect()
    }

    pub(crate) fn validate(name: &str, bytes: &[u8]) -> Result<(), Failure> {
        Self::validate_asset(Self::descriptor(name)?, bytes)
    }

    pub(crate) fn output_name(name: &str) -> String {
        format!("{}/{name}", Self::OUTPUT)
    }

    pub(crate) fn failure(path: &str, message: &str) -> Failure {
        Failure::one(diagnostic(
            1605,
            format!("Cannot use the pinned Python browser runtime: {message}"),
            Some(format!(
                "Supply verified Pyodide {} core assets under '{}'. Use the pinned hashes in docs/queue/NEXT/PYTHON-TAC.md; ty never installs or fetches Python packages automatically.",
                Self::VERSION,
                Self::cached_root().display()
            )),
            source_span(path, 0, path.len()),
        ))
    }

    fn read_assets(root: &Path) -> Result<Vec<(String, Vec<u8>)>, Failure> {
        Self::ASSETS
            .iter()
            .map(|asset| Self::read_asset(root, asset))
            .collect()
    }

    fn read_asset(root: &Path, asset: &RuntimeAsset) -> Result<(String, Vec<u8>), Failure> {
        let path = root.join(asset.name);
        Self::require_regular(&path)?;
        let mut bytes = Vec::new();
        Self::reader(&path)?
            .take((asset.bytes as u64).saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|error| {
                Self::failure(
                    asset.name,
                    &format!("cannot read {}: {error}", path.display()),
                )
            })?;
        Self::validate_asset(asset, &bytes)?;
        Ok((Self::output_path(asset), bytes))
    }

    fn require_regular(path: &Path) -> Result<(), Failure> {
        let metadata = std::fs::symlink_metadata(path).map_err(|error| {
            Self::failure(&path.to_string_lossy(), &format!("missing asset: {error}"))
        })?;
        if !metadata.file_type().is_file() {
            return Err(Self::failure(
                &path.to_string_lossy(),
                "runtime assets must be regular files",
            ));
        }
        Ok(())
    }

    fn reader(path: &Path) -> Result<std::fs::File, Failure> {
        std::fs::File::open(path).map_err(|error| {
            Self::failure(
                &path.to_string_lossy(),
                &format!("cannot open asset: {error}"),
            )
        })
    }

    fn descriptor(name: &str) -> Result<&'static RuntimeAsset, Failure> {
        Self::ASSETS
            .iter()
            .find(|asset| Self::output_path(asset) == name)
            .ok_or_else(|| Self::failure(name, "asset is not in the pinned runtime inventory"))
    }

    fn validate_asset(asset: &RuntimeAsset, bytes: &[u8]) -> Result<(), Failure> {
        Self::validate_length(asset, bytes)?;
        Self::validate_digest(asset, bytes)
    }

    fn validate_length(asset: &RuntimeAsset, bytes: &[u8]) -> Result<(), Failure> {
        if bytes.len() != asset.bytes {
            return Err(Self::failure(
                asset.name,
                "asset byte length differs from the pinned release",
            ));
        }
        Ok(())
    }

    fn validate_digest(asset: &RuntimeAsset, bytes: &[u8]) -> Result<(), Failure> {
        let digest = format!("{:x}", Sha256::digest(bytes));
        if digest != asset.sha256 {
            return Err(Self::failure(
                asset.name,
                "asset SHA256 differs from the pinned release",
            ));
        }
        Ok(())
    }

    fn output_path(asset: &RuntimeAsset) -> String {
        Self::output_name(asset.name)
    }
}
