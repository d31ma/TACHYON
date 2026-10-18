use super::config::NativeApplication;
use super::host::{first_line, native_io, native_tool_failure, run_tool};
use super::mac_os_architecture::MacOsArchitecture;
use super::mac_os_built_slice::MacOsBuiltSlice;
use super::mac_os_slice::MacOsSlice;
use super::macos::{sign_bundle, unicode_path};
use super::native_reproducibility::NativeReproducibility;
use super::rust::{Linkage, RustCompanion, RustCompanionOutput};
use super::swift_compile_inputs::SwiftCompileInputs;
use crate::Failure;
use futures_util::{StreamExt as _, TryStreamExt as _};
use std::path::{Path, PathBuf};
use tachyon_contracts::NativeTarget;

pub(super) struct MacOsExecutable<'build> {
    pub(super) application: &'build NativeApplication,
    pub(super) stage: &'build Path,
    pub(super) source: &'build Path,
    pub(super) companion: Option<&'build Path>,
    pub(super) rust_companion: Option<&'build Path>,
}

impl MacOsExecutable<'_> {
    const FRAMEWORK_ARGUMENTS: [&'static str; 8] = [
        "-framework",
        "AppKit",
        "-framework",
        "SwiftUI",
        "-framework",
        "WebKit",
        "-framework",
        "JavaScriptCore",
    ];

    pub(super) async fn build(
        &self,
        architecture: MacOsArchitecture,
        executable: &Path,
    ) -> Result<String, Failure> {
        Self::require_apple_build_machine()?;
        let helpers = Self::folder_of(executable)?;
        let placed = self.place(architecture, executable, helpers).await?;
        Self::sign_helpers(&placed.helpers).await?;
        Ok(placed.swift_version)
    }

    async fn place(
        &self,
        architecture: MacOsArchitecture,
        executable: &Path,
        helpers: &Path,
    ) -> Result<MacOsBuiltSlice, Failure> {
        if architecture.is_build_machine() {
            let project = self.stage.join("project");
            return self.build_slice(None, &project, executable, helpers).await;
        }
        self.build_slices(architecture, executable, helpers).await
    }

    async fn build_slices(
        &self,
        architecture: MacOsArchitecture,
        executable: &Path,
        helpers: &Path,
    ) -> Result<MacOsBuiltSlice, Failure> {
        let built: Vec<MacOsBuiltSlice> = futures_util::stream::iter(architecture.slices())
            .then(|slice| self.build_part(*slice))
            .try_collect()
            .await?;
        Ok(MacOsBuiltSlice {
            executable: executable.to_path_buf(),
            helpers: Self::combine(&built, executable, helpers).await?,
            swift_version: built
                .first()
                .map(|slice| slice.swift_version.clone())
                .unwrap_or_default(),
        })
    }

    async fn build_part(&self, slice: MacOsSlice) -> Result<MacOsBuiltSlice, Failure> {
        if self.rust_companion.is_some() {
            slice.require_rust_target().await?;
        }
        let folder = self.stage.join("project").join(slice.name());
        let executable = folder.join(&self.application.executable_name);
        let helpers = folder.join("helpers");
        self.build_slice(Some(slice), &folder, &executable, &helpers)
            .await
    }

    async fn build_slice(
        &self,
        slice: Option<MacOsSlice>,
        folder: &Path,
        executable: &Path,
        helpers: &Path,
    ) -> Result<MacOsBuiltSlice, Failure> {
        let library = self.compile_rust_library(slice, folder, helpers).await?;
        let swift_version = self
            .compile_swift(slice, library.as_ref(), executable)
            .await?;
        Ok(MacOsBuiltSlice {
            executable: executable.to_path_buf(),
            helpers: library
                .map(|compiled| compiled.executables)
                .unwrap_or_default(),
            swift_version,
        })
    }

    async fn compile_rust_library(
        &self,
        slice: Option<MacOsSlice>,
        folder: &Path,
        helpers: &Path,
    ) -> Result<Option<RustCompanionOutput>, Failure> {
        let Some(source) = self.rust_companion else {
            return Ok(None);
        };
        native_io(std::fs::create_dir_all(helpers), helpers)?;
        let placed = RustCompanion::compile_companion(
            self.application,
            source,
            Linkage::Static,
            slice.map(MacOsSlice::rust_target),
            &folder.join("libtachyoncompanion.a"),
            helpers,
        )
        .await?;
        Ok(Some(placed))
    }

    async fn compile_swift(
        &self,
        slice: Option<MacOsSlice>,
        library: Option<&RustCompanionOutput>,
        executable: &Path,
    ) -> Result<String, Failure> {
        let version_output = run_tool("/usr/bin/xcrun", &["swiftc", "--version"]).await?;
        let version = first_line(&version_output, "Swift unknown");
        native_io(
            std::fs::create_dir_all(Self::folder_of(executable)?),
            executable,
        )?;
        let archive = library.map(|compiled| compiled.library.as_path());
        let inputs = SwiftCompileInputs::of(self.source, self.companion, archive, executable)?;
        let target = Self::swift_target(self.application, slice);
        let mut arguments = vec!["swiftc", "-parse-as-library", "-O", "-target", &target];
        arguments.extend(Self::FRAMEWORK_ARGUMENTS);
        arguments.extend(NativeReproducibility::SWIFT_ARGUMENTS);
        arguments.extend(inputs.arguments());
        let native_arguments = library
            .into_iter()
            .flat_map(|compiled| compiled.native_libraries.arguments.iter())
            .map(String::as_str);
        arguments.extend(native_arguments);
        let folder = Some(inputs.folder.as_path());
        NativeReproducibility::run_tool_in("/usr/bin/xcrun", &arguments, folder).await?;
        Ok(version)
    }

    pub(super) fn swift_target(
        application: &NativeApplication,
        slice: Option<MacOsSlice>,
    ) -> String {
        let minimum = application.minimum_system.floor(NativeTarget::Macos);
        slice
            .unwrap_or_else(MacOsSlice::build_machine)
            .swift_target(&minimum.to_string())
    }

    async fn combine(
        built: &[MacOsBuiltSlice],
        executable: &Path,
        helpers: &Path,
    ) -> Result<Vec<PathBuf>, Failure> {
        let executables: Vec<&Path> = built
            .iter()
            .map(|slice| slice.executable.as_path())
            .collect();
        Self::lipo(&executables, executable).await?;
        let names: Vec<&std::ffi::OsStr> = built
            .first()
            .map(|slice| {
                slice
                    .helpers
                    .iter()
                    .filter_map(|path| path.file_name())
                    .collect()
            })
            .unwrap_or_default();
        let mut combined = Vec::new();
        for name in names {
            let parts: Vec<&Path> = built
                .iter()
                .filter_map(|slice| {
                    slice
                        .helpers
                        .iter()
                        .find(|path| path.file_name() == Some(name))
                })
                .map(PathBuf::as_path)
                .collect();
            let destination = helpers.join(name);
            Self::lipo(&parts, &destination).await?;
            combined.push(destination);
        }
        Ok(combined)
    }

    async fn lipo(parts: &[&Path], output: &Path) -> Result<(), Failure> {
        let output = unicode_path(output, "Application path is not valid Unicode.")?;
        let mut arguments = vec!["lipo", "-create", "-output", output];
        for part in parts {
            arguments.push(unicode_path(part, "Slice path is not valid Unicode.")?);
        }
        run_tool("/usr/bin/xcrun", &arguments).await.map(|_| ())
    }

    async fn sign_helpers(helpers: &[PathBuf]) -> Result<(), Failure> {
        for helper in helpers {
            sign_bundle(helper).await?;
        }
        Ok(())
    }

    fn folder_of(path: &Path) -> Result<&Path, Failure> {
        path.parent().ok_or_else(|| {
            native_tool_failure(1605, "The macOS executable path has no parent folder.")
        })
    }

    fn require_apple_build_machine() -> Result<(), Failure> {
        if cfg!(target_os = "macos") {
            return Ok(());
        }
        Err(native_tool_failure(
            1605,
            "The macOS host requires an Apple macOS build machine.",
        ))
    }
}
