use super::host::{native_io, native_tool_failure, write};
use super::rust::Linkage;
use super::rust_companion_crate::RustCompanionCrate;
use crate::Failure;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

pub(super) struct RustCrateBuild<'build> {
    pub(super) declaration: &'build RustCompanionCrate,
    pub(super) source: &'build Path,
    pub(super) client_root: &'build Path,
    pub(super) vendor: &'build Path,
    pub(super) target: PathBuf,
}

impl RustCrateBuild<'_> {
    pub(super) const VENDOR_FOLDER: &'static str = "vendor/rust";
    const FAILURE_CODE: u16 = 1605;
    const PROJECT_FOLDER: &'static str = "rust-companion";
    const MANIFEST: &'static str = "Cargo.toml";
    const BINARY_FOLDER: &'static str = "bin";
    pub(super) const CACHE_FOLDER: &'static str = "rust-companion";
    const RELEASE: &'static str = "release";
    const ENTRY_METHOD: &'static str = "pub fn run() -> i32";
    const EXPORTS: &'static str = "\npub use crate::TachyonRustServices::*;\n";
    const VENDOR_SOURCE: &'static str = "tachyon-vendored";
    const WINDOWS_SUFFIX: &'static str = ".exe";

    pub(super) fn stage(&self, linkage: Linkage) -> Result<PathBuf, Failure> {
        let project = self.project_folder();
        let mut binaries = Vec::with_capacity(self.declaration.executables.len());
        for (name, entry) in &self.declaration.executables {
            let main = project.join(Self::BINARY_FOLDER).join(format!("{name}.rs"));
            write(&main, self.binary_source(name, entry)?.as_bytes())?;
            binaries.push((name.clone(), main));
        }
        if !binaries.is_empty() {
            let mut source = native_io(std::fs::read_to_string(self.source), self.source)?;
            source.push_str(Self::EXPORTS);
            write(self.source, source.as_bytes())?;
        }
        let manifest = project.join(Self::MANIFEST);
        write(
            &manifest,
            self.declaration
                .manifest(self.source, linkage.cargo_crate_types(), &binaries)
                .as_bytes(),
        )?;
        Ok(manifest)
    }

    pub(super) async fn build(
        &self,
        linkage: Linkage,
        target_triple: Option<&str>,
        library: &Path,
        helpers: &Path,
    ) -> Result<Vec<PathBuf>, Failure> {
        let manifest = self.project_folder().join(Self::MANIFEST);
        self.require_vendored_crates(&manifest)?;
        let target = &self.target;
        let arguments = self.cargo_arguments(&manifest, target, target_triple)?;
        let borrowed = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        super::host::run_long_tool("cargo", &borrowed).await?;
        let built = match target_triple {
            Some(triple) => target.join(triple).join(Self::RELEASE),
            None => target.join(Self::RELEASE),
        };
        let windows = target_triple.map_or(cfg!(windows), |triple| triple.contains("windows"));
        let library_file = Self::library_file(linkage, target_triple, windows);
        Self::copy(&built.join(library_file), library)?;
        let suffix = if windows { Self::WINDOWS_SUFFIX } else { "" };
        let mut placed = Vec::with_capacity(self.declaration.executables.len());
        for name in self.declaration.executables.keys() {
            let file = format!("{name}{suffix}");
            let destination = helpers.join(&file);
            Self::copy(&built.join(&file), &destination)?;
            placed.push(destination);
        }
        Ok(placed)
    }

    fn project_folder(&self) -> PathBuf {
        self.source
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(Self::PROJECT_FOLDER)
    }

    fn binary_source(&self, name: &str, entry: &str) -> Result<String, Failure> {
        let class = RustCompanionCrate::entry_class(entry).unwrap_or_default();
        let path = self.client_root.join(entry.trim_start_matches('/'));
        let authored = std::fs::read_to_string(&path).unwrap_or_default();
        if !authored.contains(&format!("struct {class}")) || !authored.contains(Self::ENTRY_METHOD)
        {
            return Err(native_tool_failure(
                Self::FAILURE_CODE,
                &format!(
                    "The executable {name} needs client{entry} with a class {class} that has \
                     `{}`; Tachyon calls it from the executable's main function.",
                    Self::ENTRY_METHOD
                ),
            ));
        }
        let location = path.to_string_lossy();
        Ok(format!(
            "#![allow(unused_imports, non_snake_case)]\nuse {}::*;\n#[path = {location:?}]\nmod entry;\n\nfn main() {{\n    std::process::exit(entry::{class}::run());\n}}\n",
            RustCompanionCrate::LIBRARY
        ))
    }

    fn require_vendored_crates(&self, manifest: &Path) -> Result<(), Failure> {
        if !self.declaration.has_dependencies() || self.vendor.is_dir() {
            return Ok(());
        }
        Err(native_tool_failure(
            Self::FAILURE_CODE,
            &format!(
                "tac.application.rust declares crates, and Tachyon builds them offline from {}, \
                 which is missing. Run `cargo vendor --manifest-path {} {}` once, commit the \
                 folder, and build again.",
                self.vendor.display(),
                manifest.display(),
                self.vendor.display()
            ),
        ))
    }

    fn cargo_arguments(
        &self,
        manifest: &Path,
        target: &Path,
        target_triple: Option<&str>,
    ) -> Result<Vec<String>, Failure> {
        let text = |path: &Path| -> Result<String, Failure> {
            path.to_str().map(String::from).ok_or_else(|| {
                native_tool_failure(
                    Self::FAILURE_CODE,
                    "A Rust build path is not valid Unicode.",
                )
            })
        };
        let mut arguments = vec![
            String::from("build"),
            String::from("--release"),
            String::from("--offline"),
            String::from("--manifest-path"),
            text(manifest)?,
            String::from("--target-dir"),
            text(target)?,
            String::from("--config"),
            String::from("net.offline=true"),
        ];
        if let Some(triple) = target_triple {
            arguments.extend([String::from("--target"), String::from(triple)]);
        }
        if self.declaration.has_dependencies() {
            let source = Self::VENDOR_SOURCE;
            arguments.extend([
                String::from("--config"),
                format!("source.crates-io.replace-with=\"{source}\""),
                String::from("--config"),
                format!(
                    "source.{source}.directory={}",
                    RustCompanionCrate::toml_path(self.vendor)
                ),
            ]);
        }
        Ok(arguments)
    }

    fn library_file(linkage: Linkage, target_triple: Option<&str>, windows: bool) -> String {
        let library = RustCompanionCrate::LIBRARY;
        let apple =
            target_triple.map_or(cfg!(target_os = "macos"), |triple| triple.contains("apple"));
        match (linkage, windows, apple) {
            (Linkage::Static, true, _) => format!("{library}.lib"),
            (Linkage::Static, false, _) => format!("lib{library}.a"),
            (_, true, _) => format!("{library}.dll"),
            (_, false, true) => format!("lib{library}.dylib"),
            (_, false, false) => format!("lib{library}.so"),
        }
    }

    fn copy(from: &Path, to: &Path) -> Result<(), Failure> {
        if let Some(parent) = to.parent() {
            native_io(std::fs::create_dir_all(parent), parent)?;
        }
        native_io(std::fs::copy(from, to), from).map(|_| ())
    }
}
