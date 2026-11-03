use super::artifact_provenance::ArtifactProvenance;
use super::build_stage::BuildStage;
use super::content_digest::ContentDigest;
use super::portable_path::PortablePath;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tachyon_contracts::{
    ArtifactContractVersions, ArtifactManifest, ArtifactOutput, ArtifactTarget, ArtifactToolchain,
};

pub(super) struct WebArtifactManifest;

impl WebArtifactManifest {
    pub(super) const FILE: &'static str = "artifact-manifest.json";
    const OS: &'static str = "web";
    const ARCHITECTURE: &'static str = "universal";
    const ABI: &'static str = "browser";
    const TOOLCHAIN: &'static str = "rust";

    pub(super) fn write(stage: &Path, native: bool) -> io::Result<()> {
        Self::written_unless_native(stage, native)
    }

    fn written_unless_native(stage: &Path, native: bool) -> io::Result<()> {
        if native {
            return Ok(());
        }
        Self::written(stage)
    }

    fn written(stage: &Path) -> io::Result<()> {
        let manifest = Self::manifest(Self::outputs(stage)?);
        let mut bytes = serde_json::to_vec_pretty(&manifest).map_err(io::Error::other)?;
        bytes.push(b'\n');
        fs::write(stage.join(Self::FILE), bytes)
    }

    fn outputs(stage: &Path) -> io::Result<Vec<ArtifactOutput>> {
        let mut outputs = BuildStage::files(stage)?
            .iter()
            .map(|file| Self::output(stage, file))
            .collect::<io::Result<Vec<_>>>()?;
        outputs.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(outputs)
    }

    fn output(stage: &Path, file: &PathBuf) -> io::Result<ArtifactOutput> {
        let bytes = fs::read(file)?;
        let relative = file.strip_prefix(stage).map_err(io::Error::other)?;
        Ok(ArtifactOutput {
            path: PortablePath::of(relative),
            sha256: ContentDigest::sha256_bytes(&bytes),
            size: bytes.len() as u64,
        })
    }

    fn manifest(outputs: Vec<ArtifactOutput>) -> ArtifactManifest {
        ArtifactManifest {
            contract_version: 1,
            release_version: String::from(tachyon_contracts::PRODUCT_VERSION),
            commit: ArtifactProvenance::commit(),
            source_date_epoch: ArtifactProvenance::source_date_epoch(),
            target: ArtifactTarget {
                os: String::from(Self::OS),
                architecture: String::from(Self::ARCHITECTURE),
                abi: String::from(Self::ABI),
            },
            toolchains: vec![ArtifactToolchain {
                name: String::from(Self::TOOLCHAIN),
                version: String::from(ArtifactProvenance::RUST_VERSION),
            }],
            contracts: ArtifactContractVersions {
                native_host: None,
                ..ArtifactContractVersions::default()
            },
            outputs,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::WebArtifactManifest;
    use crate::{BuildOptions, WebCompiler};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        let pages = root.path().join("client/pages");
        fs::create_dir_all(&pages).expect("pages");
        fs::write(pages.join("tac.html"), "<page-ledger />").expect("page");
        let component = root.path().join("client/components/page/ledger");
        fs::create_dir_all(&component).expect("component");
        fs::write(component.join("tac.html"), "<main><h1>Ledger</h1></main>").expect("view");
        root
    }

    fn built_manifest(root: &Path, output: &str) -> Vec<u8> {
        let options = BuildOptions {
            output_directory: PathBuf::from(output),
            incremental: false,
            ..BuildOptions::default()
        };
        WebCompiler::build(root, &options).expect("build");
        fs::read(root.join(output).join(WebArtifactManifest::FILE)).expect("artifact manifest")
    }

    #[test]
    fn a_web_build_lists_every_output_in_a_valid_artifact_manifest() {
        let root = project();
        let bytes = built_manifest(root.path(), "dist");
        let manifest: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
        let contract = tachyon_contracts::find("artifact-manifest").expect("contract");
        let schema = tachyon_contracts::parse_schema(contract).expect("schema");
        let validator = jsonschema::validator_for(&schema).expect("validator");
        assert!(
            validator.is_valid(&manifest),
            "{:?}",
            validator.iter_errors(&manifest).collect::<Vec<_>>()
        );
        assert_eq!(manifest["target"]["os"], "web");
        assert!(manifest["contracts"].get("native_host").is_none());
        let paths: Vec<&str> = manifest["outputs"]
            .as_array()
            .expect("outputs")
            .iter()
            .filter_map(|output| output["path"].as_str())
            .collect();
        assert!(paths.contains(&"index.html"), "{paths:?}");
        assert!(paths.contains(&"route-manifest.json"), "{paths:?}");
        assert!(!paths.contains(&WebArtifactManifest::FILE), "{paths:?}");
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        assert_eq!(paths, sorted);
        let listed = manifest["outputs"].as_array().map_or(0, Vec::len);
        let written = crate::compiler::BuildStage::files(&root.path().join("dist"))
            .expect("files")
            .len();
        assert_eq!(listed + 1, written, "every file but the manifest is listed");
    }

    #[test]
    fn two_web_builds_of_the_same_sources_write_the_same_artifact_manifest() {
        let root = project();
        assert_eq!(
            built_manifest(root.path(), "dist"),
            built_manifest(root.path(), "dist-again")
        );
    }
}
