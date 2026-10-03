use super::{CapturedFile, RoleSource};
use crate::Failure;
use crate::annotation_layout::{BrowserInteropCheck, CohesionCheck, RuleSwitch};
use crate::compiler::shared_scripts::SharedScripts;
use crate::failure::{diagnostic, source_span};
use crate::native::SiteSettings;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct CaptureVendor {
    paths: BTreeSet<PathBuf>,
}

impl CaptureVendor {
    const MANIFEST: &'static str = "client/manifest.json";
    const SHARED: &'static str = "client/shared";

    pub(super) fn from_files(
        configs: &[CapturedFile],
        shared: &[CapturedFile],
    ) -> Result<Self, Failure> {
        let manifest = Self::manifest_bytes(configs);
        let site = SiteSettings::from_captured_manifest(manifest)?;
        let paths = Self::verified_paths(&site, shared)?;
        Ok(Self { paths })
    }

    pub(super) fn check_file(
        &self,
        file: &CapturedFile,
        cohesion: RuleSwitch,
    ) -> Result<(), Failure> {
        Self::role_check(file, self.paths.contains(&file.relative))?;
        BrowserInteropCheck::check_file(&file.relative, &file.bytes)?;
        CohesionCheck::check_file(&file.relative, &file.bytes, cohesion)
    }

    fn manifest_bytes(configs: &[CapturedFile]) -> Option<&[u8]> {
        configs
            .iter()
            .find(|file| file.relative == Path::new(Self::MANIFEST))
            .map(|file| file.bytes.as_slice())
    }

    fn verified_paths(
        site: &SiteSettings,
        shared: &[CapturedFile],
    ) -> Result<BTreeSet<PathBuf>, Failure> {
        site.script_sources
            .keys()
            .map(|url| Self::verified_path(site, shared, url))
            .collect()
    }

    fn verified_path(
        site: &SiteSettings,
        shared: &[CapturedFile],
        url: &str,
    ) -> Result<PathBuf, Failure> {
        let path = Path::new("client").join(url.trim_start_matches('/'));
        let file = shared
            .iter()
            .find(|file| file.relative == path)
            .ok_or_else(|| Self::missing(&path))?;
        Self::require_shared(&file.relative)?;
        let verified = SharedScripts::verified_vendor(site, &file.relative, &file.bytes)?;
        verified
            .then(|| file.relative.clone())
            .ok_or_else(|| Self::missing(&path))
    }

    fn require_shared(path: &Path) -> Result<(), Failure> {
        if !path.starts_with(Self::SHARED) {
            return Err(Self::missing(path));
        }
        Ok(())
    }

    fn role_check(file: &CapturedFile, vendor: bool) -> Result<(), Failure> {
        if vendor {
            return RoleSource::validate_verified_vendor(file);
        }
        RoleSource::validate(file)
    }

    fn missing(path: &Path) -> Failure {
        let source = path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(1364, "Declared script vendor is missing from captured shared resources.", Some("Declare only an exact captured third-party resource in tac.scripts and tac.scriptSources with its package, pinned version, license and SHA-256.".to_owned()), source_span(&source, 0, 0)))
    }
}
