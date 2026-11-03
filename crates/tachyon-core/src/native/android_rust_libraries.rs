use super::android_architecture::AndroidArchitecture;
use super::android_ndk::AndroidNdk;
use super::config::NativeApplication;
use super::rust::RustCompanion;
use super::rust_standard_library::RustStandardLibrary;
use crate::Failure;
use std::path::{Path, PathBuf};
use tachyon_contracts::NativeTarget;

pub(super) struct AndroidRustLibraries;

impl AndroidRustLibraries {
    pub(super) const LIBRARY: &'static str = "tachyoncompanion";
    const LIBRARY_FOLDER: &'static str = "app/src/main/jniLibs";
    const NEEDED_BY: &'static str = "Rust services in an Android app";

    pub(super) async fn build(
        application: &NativeApplication,
        source: &Path,
        (project, architecture): (&Path, AndroidArchitecture),
    ) -> Result<(), Failure> {
        let ndk = AndroidNdk::from_environment(|name| std::env::var_os(name))?;
        let api_level = application
            .minimum_system
            .floor(NativeTarget::Android)
            .major();
        Self::build_each_abi(
            application,
            source,
            (project, architecture),
            (&ndk, api_level),
        )
        .await
    }

    async fn build_each_abi(
        application: &NativeApplication,
        source: &Path,
        (project, architecture): (&Path, AndroidArchitecture),
        (ndk, api_level): (&AndroidNdk, u32),
    ) -> Result<(), Failure> {
        for (abi, triple) in architecture.abis() {
            RustStandardLibrary::require(triple, Self::NEEDED_BY).await?;
            let linker = ndk.linker(triple, api_level)?;
            let library = Self::library_path(project, abi);
            RustCompanion::compile_for_android(application, source, (triple, &linker), &library)
                .await?;
        }
        Ok(())
    }

    pub(super) fn library_path(project: &Path, abi: &str) -> PathBuf {
        project
            .join(Self::LIBRARY_FOLDER)
            .join(abi)
            .join(format!("lib{}.so", Self::LIBRARY))
    }
}
