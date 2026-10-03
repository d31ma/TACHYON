use super::{NativeBuildOptions, NativeCompiler, build_web_bundle};
use crate::native::native_web_stage::NativeWebStage;
use crate::{BuildOptions, ProjectDiscovery, WebCompiler};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tachyon_contracts::NativeTarget;

struct NativeExternalOutputTests {
    workspace: tempfile::TempDir,
}

impl NativeExternalOutputTests {
    const COMPONENT: &'static str = "client/components/native/output/tac.html";
    const MANIFEST: &'static str = r#"{"name":"External Output","start_url":"/","tac":{"application":{"id":"dev.tachyon.external-output","version":"1.0.0"}}}"#;

    fn new(view: &str) -> io::Result<Self> {
        let temporary_root = fs::canonicalize(std::env::temp_dir())?;
        let fixture = Self {
            workspace: tempfile::Builder::new().tempdir_in(temporary_root)?,
        };
        let source = fixture.source_root();
        Self::write(&source, "client/manifest.json", Self::MANIFEST.as_bytes())?;
        Self::write(&source, "client/pages/tac.html", b"<native-output />")?;
        Self::write(&source, Self::COMPONENT, view.as_bytes())?;
        Ok(fixture)
    }

    fn source_root(&self) -> PathBuf {
        self.workspace.path().join("authored-source")
    }

    fn output_root(&self) -> PathBuf {
        self.workspace.path().join("external-output")
    }

    fn options(&self) -> NativeBuildOptions {
        NativeBuildOptions {
            output_directory: self.output_root(),
            target: NativeTarget::Macos,
            package: false,
        }
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> io::Result<()> {
        let destination = root.join(relative);
        let parent = destination
            .parent()
            .ok_or_else(|| io::Error::other("fixture path needs a parent"))?;
        fs::create_dir_all(parent)?;
        fs::write(destination, bytes)
    }

    fn inventory(root: &Path) -> io::Result<BTreeMap<PathBuf, Option<Vec<u8>>>> {
        let mut inventory = BTreeMap::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(path) = pending.pop() {
            let relative = path
                .strip_prefix(root)
                .map_err(io::Error::other)?
                .to_path_buf();
            if fs::metadata(&path)?.is_dir() {
                inventory.insert(relative, None);
                pending.extend(
                    fs::read_dir(path)?
                        .map(|entry| entry.map(|entry| entry.path()))
                        .collect::<io::Result<Vec<_>>>()?,
                );
            } else {
                inventory.insert(relative, Some(fs::read(path)?));
            }
        }
        Ok(inventory)
    }

    fn output_names(root: &Path) -> io::Result<Vec<String>> {
        fs::read_dir(root)?
            .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
            .collect()
    }
}

#[tokio::test]
async fn native_web_bundle_keeps_live_staging_and_generated_types_outside_authored_source()
-> Result<(), Box<dyn Error>> {
    let fixture = NativeExternalOutputTests::new("<main>External staging</main>")?;
    let before = NativeExternalOutputTests::inventory(&fixture.source_root())?;
    let project = ProjectDiscovery::discover(fixture.source_root())?;
    let (web, companions) =
        build_web_bundle(&project, NativeTarget::Macos, &fixture.output_root()).await?;
    let stage = web.path().to_path_buf();
    assert_eq!(stage.parent(), Some(fixture.output_root().as_path()));
    assert!(!stage.starts_with(fixture.source_root()));
    assert!(stage.join("index.html").is_file());
    assert!(stage.join(".tachyon/tac-client.js").is_file());
    assert!(companions.is_empty());
    assert!(
        project
            .snapshot_root()
            .join(".tachyon/types/client/composed.d.ts")
            .is_file()
    );
    assert!(!fixture.source_root().join(".tachyon").exists());
    assert_eq!(
        NativeExternalOutputTests::inventory(&fixture.source_root())?,
        before
    );
    drop(web);
    assert!(!stage.exists());
    assert!(NativeExternalOutputTests::output_names(&fixture.output_root())?.is_empty());
    Ok(())
}

#[tokio::test]
async fn public_native_build_promotes_external_output_without_changing_authored_inventory()
-> Result<(), Box<dyn Error>> {
    let fixture = NativeExternalOutputTests::new("<main>External staging</main>")?;
    let before = NativeExternalOutputTests::inventory(&fixture.source_root())?;
    let result = NativeCompiler::build_async(fixture.source_root(), &fixture.options()).await?;
    assert_eq!(
        result.output_directory(),
        fixture.output_root().join("macos")
    );
    assert!(result.application_bundle().is_file());
    assert!(result.output_directory().join("web/index.html").is_file());
    assert!(
        result
            .output_directory()
            .join("artifact-manifest.json")
            .is_file()
    );
    assert_eq!(result.route_count(), 1);
    assert_eq!(
        NativeExternalOutputTests::inventory(&fixture.source_root())?,
        before
    );
    assert_eq!(
        NativeExternalOutputTests::output_names(&fixture.output_root())?,
        ["macos"]
    );
    Ok(())
}

#[tokio::test]
async fn failed_native_web_build_cleans_external_scratch_and_preserves_source_and_prior_output()
-> Result<(), Box<dyn Error>> {
    let fixture = NativeExternalOutputTests::new("<main>{bad syntax}</main>")?;
    let before = NativeExternalOutputTests::inventory(&fixture.source_root())?;
    NativeExternalOutputTests::write(
        &fixture.output_root(),
        "macos/accepted.txt",
        b"previous accepted artifact",
    )?;
    let failure = NativeCompiler::build_async(fixture.source_root(), &fixture.options())
        .await
        .err()
        .ok_or_else(|| io::Error::other("invalid captured view must fail"))?;
    assert!(failure.diagnostics().iter().any(|diagnostic| {
        diagnostic
            .spans
            .iter()
            .any(|span| span.file == NativeExternalOutputTests::COMPONENT)
    }));
    assert_eq!(
        fs::read(fixture.output_root().join("macos/accepted.txt"))?,
        b"previous accepted artifact"
    );
    assert_eq!(
        NativeExternalOutputTests::output_names(&fixture.output_root())?,
        ["macos"]
    );
    assert_eq!(
        NativeExternalOutputTests::inventory(&fixture.source_root())?,
        before
    );
    Ok(())
}

#[tokio::test]
async fn ordinary_web_build_retains_authored_editor_declaration_behavior()
-> Result<(), Box<dyn Error>> {
    let fixture = NativeExternalOutputTests::new("<main>Ordinary web</main>")?;
    let result = WebCompiler::build_async(
        fixture.source_root(),
        &BuildOptions {
            output_directory: fixture.output_root(),
            ..BuildOptions::default()
        },
    )
    .await?;
    assert!(result.output_directory().join("index.html").is_file());
    assert!(
        fixture
            .source_root()
            .join(".tachyon/types/client/composed.d.ts")
            .is_file()
    );
    assert!(
        fixture
            .source_root()
            .join(".tachyon/types/server/composed.d.ts")
            .is_file()
    );
    Ok(())
}

#[test]
fn native_web_staging_refuses_an_external_regular_file_without_overwriting_it()
-> Result<(), Box<dyn Error>> {
    let fixture = NativeExternalOutputTests::new("<main>External staging</main>")?;
    fs::write(fixture.output_root(), b"owned file")?;
    let failure = NativeWebStage::create(&fixture.output_root())
        .err()
        .ok_or_else(|| io::Error::other("regular file cannot become native scratch"))?;
    assert!(failure.to_string().contains("TY1605"));
    assert!(
        failure
            .to_string()
            .contains(&fixture.output_root().display().to_string())
    );
    assert_eq!(fs::read(fixture.output_root())?, b"owned file");
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn public_native_build_refuses_a_symlinked_external_output_without_touching_source()
-> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::symlink;

    let fixture = NativeExternalOutputTests::new("<main>External staging</main>")?;
    let before = NativeExternalOutputTests::inventory(&fixture.source_root())?;
    let target = fixture.workspace.path().join("symlink-target");
    fs::create_dir(&target)?;
    symlink(&target, fixture.output_root())?;
    let failure = NativeCompiler::build_async(fixture.source_root(), &fixture.options())
        .await
        .err()
        .ok_or_else(|| io::Error::other("symlinked output must fail"))?;
    assert!(failure.to_string().contains("TY1202"));
    assert!(fs::read_dir(target)?.next().is_none());
    assert_eq!(
        NativeExternalOutputTests::inventory(&fixture.source_root())?,
        before
    );
    Ok(())
}
