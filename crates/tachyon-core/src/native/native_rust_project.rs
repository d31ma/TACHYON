use super::rust_companion_crate::RustCompanionCrate;
use super::rust_crate_build::RustCrateBuild;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct NativeRustProject {
    pub(super) declaration: RustCompanionCrate,
    pub(super) client_root: PathBuf,
    pub(super) vendor: PathBuf,
}

impl NativeRustProject {
    const CLIENT_FOLDER: &'static str = "client";

    pub(super) fn located(
        declaration: RustCompanionCrate,
        source_root: &Path,
        authored_root: &Path,
    ) -> Self {
        Self {
            declaration,
            client_root: source_root.join(Self::CLIENT_FOLDER),
            vendor: authored_root.join(RustCrateBuild::VENDOR_FOLDER),
        }
    }

    pub(super) fn crate_build<'build>(
        &'build self,
        source: &'build Path,
        cache_key: &'build str,
    ) -> RustCrateBuild<'build> {
        RustCrateBuild {
            declaration: &self.declaration,
            source,
            client_root: &self.client_root,
            vendor: &self.vendor,
            target: crate::cache::root()
                .join(RustCrateBuild::CACHE_FOLDER)
                .join(cache_key),
        }
    }
}
