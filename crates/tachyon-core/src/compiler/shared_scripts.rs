use super::shared_assets::SharedAssets;
use crate::Failure;
use crate::annotation_layout::AuthoredDomGuard;
use crate::failure::{diagnostic, source_span};
use crate::native::SiteSettings;
use std::path::{Path, PathBuf};

pub(crate) struct SharedScripts;

impl SharedScripts {
    const CODE: u16 = 1364;

    pub(super) fn prepare(
        root: &Path,
        assets: Vec<(PathBuf, Vec<u8>)>,
    ) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        let site = SiteSettings::load(root)?;
        Self::require_vendor_files(&site, &assets)?;
        Self::prepare_assets(&site, assets)
    }

    pub(super) fn check_file(root: &Path, path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        let site = SiteSettings::load(root)?;
        Self::check_source(&site, path, bytes)
    }

    pub(crate) fn verified_vendor(
        site: &SiteSettings,
        path: &Path,
        bytes: &[u8],
    ) -> Result<bool, Failure> {
        Self::verify_vendor(site, path, bytes)
    }

    fn prepare_assets(
        site: &SiteSettings,
        assets: Vec<(PathBuf, Vec<u8>)>,
    ) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        assets
            .into_iter()
            .map(|(path, bytes)| Self::prepare_asset(site, path, bytes))
            .collect()
    }

    fn prepare_asset(
        site: &SiteSettings,
        path: PathBuf,
        bytes: Vec<u8>,
    ) -> Result<(PathBuf, Vec<u8>), Failure> {
        let source = PathBuf::from("client").join(&path);
        let vendor = Self::verify_vendor(site, &source, &bytes)?;
        Self::check_authored(vendor, &source, &bytes)?;
        let bytes = Self::published_bytes(vendor, &path, bytes);
        Ok((path, bytes))
    }

    fn check_source(site: &SiteSettings, path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        let vendor = Self::verify_vendor(site, path, bytes)?;
        Self::check_authored(vendor, path, bytes)
    }

    fn verify_vendor(site: &SiteSettings, path: &Path, bytes: &[u8]) -> Result<bool, Failure> {
        let relative = path
            .strip_prefix("client")
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let url = format!("/{relative}");
        site.script_sources.get(&url).map_or(Ok(false), |vendor| {
            Self::require_vendor_path(path)?;
            Self::require_script(site, &url, path)?;
            vendor.verify(&path.to_string_lossy().replace('\\', "/"), bytes)?;
            Ok(true)
        })
    }

    fn require_vendor_path(path: &Path) -> Result<(), Failure> {
        let shared = path.starts_with("client/shared");
        let utility = crate::annotation_layout::UtilityLayout::is_client_utility(path);
        if [!shared, utility].contains(&true) {
            return Err(Self::failure(
                path,
                "Script vendor exceptions require captured client/shared resources outside the registered utility folders.",
            ));
        }
        Ok(())
    }

    fn require_script(site: &SiteSettings, url: &str, path: &Path) -> Result<(), Failure> {
        if !site.scripts.iter().any(|script| script == url) {
            return Err(Self::failure(
                path,
                "Every tac.scriptSources vendor must also appear in tac.scripts.",
            ));
        }
        Ok(())
    }

    fn require_vendor_files(
        site: &SiteSettings,
        assets: &[(PathBuf, Vec<u8>)],
    ) -> Result<(), Failure> {
        site.script_sources
            .keys()
            .try_for_each(|url| Self::require_vendor_file(url, assets))
    }

    fn require_vendor_file(url: &str, assets: &[(PathBuf, Vec<u8>)]) -> Result<(), Failure> {
        let path = Path::new(url.trim_start_matches('/'));
        if !assets.iter().any(|(asset, _)| asset == path) {
            return Err(Self::failure(
                &PathBuf::from("client").join(path),
                "Declared script vendor is missing from captured client/shared resources.",
            ));
        }
        Ok(())
    }

    fn check_authored(vendor: bool, path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        (!vendor)
            .then_some(())
            .map_or(Ok(()), |_| AuthoredDomGuard::check_bytes(path, bytes))
    }

    fn published_bytes(vendor: bool, path: &Path, bytes: Vec<u8>) -> Vec<u8> {
        if vendor {
            return bytes;
        }
        SharedAssets::strip_css_imports(path, bytes)
    }

    fn failure(path: &Path, message: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(Self::CODE, message, Some("Declare only exact pinned third-party resource paths in tac.scripts and tac.scriptSources; keep authored app markup and styles in component files.".to_owned()), source_span(&portable, 0, 0)))
    }
}
