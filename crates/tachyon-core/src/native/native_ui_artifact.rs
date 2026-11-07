use super::{
    compiler::NativeBuildResult, native_ui_capture::NativeUiCapture,
    native_ui_contract::NativeUiContract, native_ui_descriptor::NativeUiDescriptor,
    native_ui_generation::NativeUiGeneration, native_ui_problem::NativeUiProblem,
};
use crate::Failure;
use cap_std::{ambient_authority, fs::Dir};
use sha2::{Digest, Sha256};
use std::path::Path;
use tachyon_contracts::{ArtifactManifest, ArtifactOutput};

pub(super) struct NativeUiArtifact {
    capture: NativeUiCapture,
    manifest: ArtifactManifest,
}

impl NativeUiArtifact {
    const MANIFEST: &'static str = "artifact-manifest.json";

    pub(super) fn generation(build: &NativeBuildResult) -> Result<NativeUiGeneration, Failure> {
        let root = Dir::open_ambient_dir(build.output_directory(), ambient_authority())
            .map_err(|error| NativeUiProblem::io(&error))?;
        let capture = NativeUiCapture::artifact(&root)?;
        Self::integrity(&capture, build.sha256())?;
        let artifact = Self::read(capture)?;
        artifact.generation_from_artifact()
    }

    fn read(capture: NativeUiCapture) -> Result<Self, Failure> {
        let source = capture.bytes.get(Self::MANIFEST).ok_or_else(|| {
            NativeUiProblem::failure("Native update requires the generated artifact manifest.")
        })?;
        Self::manifest_size(source)?;
        let manifest = serde_json::from_slice(source).map_err(|error| {
            NativeUiProblem::failure(&format!("Invalid native artifact manifest: {error}"))
        })?;
        let artifact = Self { capture, manifest };
        artifact.validate()?;
        Ok(artifact)
    }

    fn generation_from_artifact(&self) -> Result<NativeUiGeneration, Failure> {
        let resource_prefix = self.resource_prefix()?;
        let native_sha256 = self.native_fingerprint(&resource_prefix)?;
        let contract = NativeUiContract::macos(self.application_id()?, native_sha256)?;
        NativeUiGeneration::captured(self.capture.subtree(&resource_prefix)?, contract)
    }

    fn validate(&self) -> Result<(), Failure> {
        Self::target(&self.manifest)?;
        Self::contracts(&self.manifest)?;
        NativeUiProblem::require(
            self.recorded_outputs() == self.manifest.outputs,
            "Native output does not match its complete artifact manifest.",
        )
    }

    fn recorded_outputs(&self) -> Vec<ArtifactOutput> {
        self.capture
            .assets
            .iter()
            .filter(|asset| asset.path != Self::MANIFEST)
            .map(|asset| ArtifactOutput {
                path: asset.path.clone(),
                size: asset.size,
                sha256: asset.sha256.clone(),
            })
            .collect()
    }

    fn resource_prefix(&self) -> Result<String, Failure> {
        let applications: Vec<_> = self
            .capture
            .assets
            .iter()
            .filter(|asset| Self::is_application_info(&asset.path))
            .collect();
        Self::one_application(applications.len())?;
        let contents = Path::new(&applications[0].path).parent().ok_or_else(|| {
            NativeUiProblem::failure("Native update application resources are missing.")
        })?;
        Ok(format!(
            "{}/Resources/WebBundle/",
            contents.to_string_lossy()
        ))
    }

    fn is_application_info(path: &str) -> bool {
        let path = Path::new(path);
        let application = path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::extension);
        [
            path.ends_with("Contents/Info.plist"),
            application == Some(std::ffi::OsStr::new("app")),
        ]
        .into_iter()
        .all(|valid| valid)
    }

    fn one_application(count: usize) -> Result<(), Failure> {
        NativeUiProblem::require(
            count == 1,
            "Native update requires exactly one generated macOS application.",
        )
    }

    fn native_fingerprint(&self, resources: &str) -> Result<String, Failure> {
        let outputs: Vec<_> = self
            .capture
            .assets
            .iter()
            .filter(|asset| !Self::is_ui_envelope(&asset.path, resources))
            .collect();
        let source = serde_json::to_vec(&(
            &self.manifest.target,
            &self.manifest.toolchains,
            &self.manifest.contracts,
            outputs,
        ))
        .map_err(|error| {
            NativeUiProblem::failure(&format!(
                "Cannot identify installed native contract: {error}"
            ))
        })?;
        Ok(NativeUiDescriptor::sha256(&source))
    }

    fn is_ui_envelope(path: &str, resources: &str) -> bool {
        let application = resources
            .strip_suffix("Resources/WebBundle/")
            .unwrap_or(resources);
        [
            path == Self::MANIFEST,
            Path::new(path).starts_with("web"),
            path.starts_with(resources),
            path.starts_with(&format!("{application}_CodeSignature/")),
        ]
        .contains(&true)
    }

    fn application_id(&self) -> Result<String, Failure> {
        let host = self.document("tachyon.host.json")?;
        NativeUiProblem::require(
            host["schemaVersion"] == 3
                && host["platformApiVersion"] == 1
                && host["bridgeVersion"] == 3,
            "Unknown generated native host or bridge contract.",
        )?;
        let index = self.document("native-index.json")?;
        NativeUiProblem::require(
            index["contract_version"] == 2,
            "Unknown generated native route contract.",
        )?;
        host["appId"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| NativeUiProblem::failure("Native host has no application identity."))
    }

    fn document(&self, path: &str) -> Result<serde_json::Value, Failure> {
        let source = self
            .capture
            .bytes
            .get(path)
            .ok_or_else(|| NativeUiProblem::failure("Native update metadata is missing."))?;
        Self::manifest_size(source)?;
        serde_json::from_slice(source).map_err(|error| {
            NativeUiProblem::failure(&format!("Invalid native update metadata: {error}"))
        })
    }

    fn integrity(capture: &NativeUiCapture, expected: &str) -> Result<(), Failure> {
        let mut hash = Sha256::new();
        for asset in &capture.assets {
            hash.update(asset.path.as_bytes());
            hash.update([0]);
            hash.update(asset.sha256.as_bytes());
            hash.update([0]);
        }
        let actual = crate::compiler::hex_digest(hash.finalize());
        NativeUiProblem::require(
            actual == expected,
            "Native build changed after its output fingerprint was recorded.",
        )
    }

    fn target(manifest: &ArtifactManifest) -> Result<(), Failure> {
        NativeUiProblem::require(
            manifest.target.os == "macos" && manifest.target.abi == "wkwebview",
            "Native update artifact must target macOS WKWebView.",
        )
    }

    fn contracts(manifest: &ArtifactManifest) -> Result<(), Failure> {
        NativeUiProblem::require(
            manifest.contract_version == 1
                && manifest.contracts.artifact_manifest == 1
                && manifest.contracts.native_host == Some(3),
            "Unknown native artifact compatibility contract.",
        )
    }

    fn manifest_size(bytes: &[u8]) -> Result<(), Failure> {
        NativeUiProblem::require(
            bytes.len() <= NativeUiDescriptor::MAX_BYTES,
            "Native update metadata exceeds 1 MiB.",
        )
    }
}
