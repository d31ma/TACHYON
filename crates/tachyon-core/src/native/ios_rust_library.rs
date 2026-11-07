use super::config::NativeApplication;
use super::ios_destination::IosDestination;
use super::rust::{Linkage, RustCompanion, RustCompanionOutput};
use super::rust_standard_library::RustStandardLibrary;
use crate::Failure;
use std::path::Path;

pub(super) struct IosRustLibrary;

impl IosRustLibrary {
    const NEEDED_BY: &'static str = "The iOS app's Rust services";
    const LIBRARY: &'static str = "libtachyoncompanion.a";

    pub(super) async fn compile(
        application: &NativeApplication,
        source: Option<&Path>,
        destination: IosDestination,
        project: &Path,
    ) -> Result<Option<RustCompanionOutput>, Failure> {
        let compiled =
            source.map(|source| Self::compile_source(application, source, destination, project));
        futures_util::future::OptionFuture::from(compiled)
            .await
            .transpose()
    }

    async fn compile_source(
        application: &NativeApplication,
        source: &Path,
        destination: IosDestination,
        project: &Path,
    ) -> Result<RustCompanionOutput, Failure> {
        let target = destination.rust_target();
        RustStandardLibrary::require(target, Self::NEEDED_BY).await?;
        RustCompanion::compile_companion(
            application,
            source,
            Linkage::Static,
            Some(target),
            &project.join(Self::LIBRARY),
            &project.join("helpers"),
        )
        .await
    }
}
