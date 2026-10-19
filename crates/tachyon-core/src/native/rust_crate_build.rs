use super::host::{native_io, native_tool_failure, write};
use super::rust::{Linkage, RustCompanionOutput};
use super::rust_companion_crate::RustCompanionCrate;
use super::rust_native_libraries::RustNativeLibraries;
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
    pub(super) linker: Option<PathBuf>,
}

impl RustCrateBuild<'_> {
    pub(super) const VENDOR_FOLDER: &'static str = "vendor/rust";
    pub(super) const CACHE_FOLDER: &'static str = "rust-companion";
    const FAILURE_CODE: u16 = 1605;
    const PROJECT_FOLDER: &'static str = "rust-companion";
    const MANIFEST: &'static str = "Cargo.toml";
    const BINARY_FOLDER: &'static str = "bin";
    const ENTRY_METHOD: &'static str = "pub fn run() -> i32";
    const ENTRY_SUFFIX: &'static str = "-entry.rs";
    const PARENT_FOLDER: &'static str = "..";
    const EXPORTS: &'static str = "\npub use crate::TachyonRustServices::*;\n";
    const VENDOR_SOURCE: &'static str = "tachyon-vendored";

    pub(super) fn stage(&self, linkage: Linkage) -> Result<PathBuf, Failure> {
        let project = self.project_folder();
        let binaries = self
            .declaration
            .executables
            .iter()
            .map(|(name, entry)| self.stage_binary(&project, name, entry))
            .collect::<Result<Vec<_>, Failure>>()?;
        self.export_for_executables(&binaries)?;
        let manifest = project.join(Self::MANIFEST);
        let library = PathBuf::from(format!(
            "{}/{}",
            Self::PARENT_FOLDER,
            self.source
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ));
        let text = self
            .declaration
            .manifest(&library, Self::crate_types(linkage), &binaries);
        write(&manifest, text.as_bytes())?;
        Ok(manifest)
    }

    pub(super) async fn build(
        &self,
        linkage: Linkage,
        target_triple: Option<&str>,
        library: &Path,
        helpers: &Path,
    ) -> Result<RustCompanionOutput, Failure> {
        let manifest = self.project_folder().join(Self::MANIFEST);
        self.require_vendored_crates(&manifest)?;
        let arguments = self.cargo_arguments(&manifest, &self.target, target_triple)?;
        let borrowed = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        super::host::NativeHost::run_long_tool("cargo", &borrowed).await?;
        let native_libraries = self
            .native_libraries(linkage, &manifest, target_triple)
            .await?;
        let built = target_triple.map_or_else(
            || self.target.join(Self::RELEASE),
            |triple| self.target.join(triple).join(Self::RELEASE),
        );
        let windows = target_triple.map_or(cfg!(windows), |triple| triple.contains(Self::WINDOWS));
        let library_file = Self::library_file(linkage, target_triple, windows);
        Self::copy(&built.join(library_file), library)?;
        let suffix = Self::EXECUTABLE_SUFFIXES[usize::from(windows)];
        let executables = self
            .declaration
            .executables
            .keys()
            .map(|name| Self::place_helper(&built, helpers, &format!("{name}{suffix}")))
            .collect::<Result<Vec<_>, Failure>>()?;
        Ok(RustCompanionOutput {
            library: library.to_path_buf(),
            executables,
            native_libraries,
        })
    }

    async fn native_libraries(
        &self,
        linkage: Linkage,
        manifest: &Path,
        target_triple: Option<&str>,
    ) -> Result<RustNativeLibraries, Failure> {
        if linkage != Linkage::Static {
            return Ok(RustNativeLibraries::default());
        }
        let arguments = self.native_library_arguments(manifest, target_triple)?;
        let borrowed = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        let output = super::host::NativeHost::run_long_tool_output("cargo", &borrowed).await?;
        RustNativeLibraries::read(&output)
    }

    fn native_library_arguments(
        &self,
        manifest: &Path,
        target_triple: Option<&str>,
    ) -> Result<Vec<String>, Failure> {
        let mut arguments = self.cargo_arguments(manifest, &self.target, target_triple)?;
        arguments[0] = String::from("rustc");
        arguments.extend(["--lib", "--", "--print=native-static-libs"].map(String::from));
        Ok(arguments)
    }

    fn project_folder(&self) -> PathBuf {
        self.source
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(Self::PROJECT_FOLDER)
    }

    fn stage_binary(
        &self,
        project: &Path,
        name: &str,
        entry: &str,
    ) -> Result<(String, PathBuf), Failure> {
        let folder = project.join(Self::BINARY_FOLDER);
        let authored = self.entry_source(name, entry)?;
        write(
            &folder.join(format!("{name}{}", Self::ENTRY_SUFFIX)),
            authored.as_bytes(),
        )?;
        let main = PathBuf::from(format!("{}/{name}.rs", Self::BINARY_FOLDER));
        write(
            &project.join(&main),
            Self::binary_source(name, entry).as_bytes(),
        )?;
        Ok((name.to_owned(), main))
    }

    fn export_for_executables(&self, binaries: &[(String, PathBuf)]) -> Result<(), Failure> {
        if binaries.is_empty() {
            return Ok(());
        }
        let mut source = native_io(std::fs::read_to_string(self.source), self.source)?;
        source.push_str(Self::EXPORTS);
        write(self.source, source.as_bytes())
    }

    fn place_helper(built: &Path, helpers: &Path, file: &str) -> Result<PathBuf, Failure> {
        let destination = helpers.join(file);
        Self::copy(&built.join(file), &destination)?;
        Ok(destination)
    }

    fn entry_source(&self, name: &str, entry: &str) -> Result<String, Failure> {
        let class = RustCompanionCrate::entry_class(entry).unwrap_or_default();
        let path = self.client_root.join(entry.trim_start_matches('/'));
        let authored = std::fs::read_to_string(&path).unwrap_or_default();
        let declares_class = authored.contains(&format!("struct {class}"));
        (declares_class & authored.contains(Self::ENTRY_METHOD))
            .then_some(())
            .ok_or_else(|| Self::missing_entry(name, entry, class))?;
        Ok(authored)
    }

    fn binary_source(name: &str, entry: &str) -> String {
        let class = RustCompanionCrate::entry_class(entry).unwrap_or_default();
        format!(
            "#![allow(unused_imports, non_snake_case)]\nuse {}::*;\n#[path = \"{name}{}\"]\nmod entry;\n\nfn main() {{\n    std::process::exit(entry::{class}::run());\n}}\n",
            RustCompanionCrate::LIBRARY,
            Self::ENTRY_SUFFIX
        )
    }

    fn missing_entry(name: &str, entry: &str, class: &str) -> Failure {
        native_tool_failure(
            Self::FAILURE_CODE,
            &format!(
                "The executable {name} needs client{entry} with a class {class} that has \
                 `{}`; Tachyon calls it from the executable's main function.",
                Self::ENTRY_METHOD
            ),
        )
    }

    fn require_vendored_crates(&self, manifest: &Path) -> Result<(), Failure> {
        let ready = !self.declaration.has_dependencies() | self.vendor.is_dir();
        ready.then_some(()).ok_or_else(|| {
            native_tool_failure(
                Self::FAILURE_CODE,
                &format!(
                    "tac.application.rust declares crates, and Tachyon builds them offline from \
                     {}, which is missing. Stage the app once with the same target and --native, \
                     then run `cargo vendor --manifest-path <out-dir>/<target>/project/{}/{} {}` \
                     (the only step that downloads), keep the folder with the project, and build \
                     again.",
                    self.vendor.display(),
                    Self::PROJECT_FOLDER,
                    manifest.file_name().unwrap_or_default().to_string_lossy(),
                    self.vendor.display()
                ),
            )
        })
    }

    fn cargo_arguments(
        &self,
        manifest: &Path,
        target: &Path,
        target_triple: Option<&str>,
    ) -> Result<Vec<String>, Failure> {
        let mut arguments = vec![
            String::from("build"),
            String::from("--release"),
            String::from("--offline"),
            String::from("--manifest-path"),
            Self::text(manifest)?,
            String::from("--target-dir"),
            Self::text(target)?,
            String::from("--config"),
            String::from("net.offline=true"),
        ];
        arguments.extend(
            target_triple
                .into_iter()
                .flat_map(|triple| [String::from("--target"), String::from(triple)]),
        );
        arguments.extend(self.vendor_arguments());
        arguments.extend(self.linker_arguments(target_triple));
        Ok(arguments)
    }

    fn linker_arguments(&self, target_triple: Option<&str>) -> Vec<String> {
        let Some((triple, linker)) = target_triple.zip(self.linker.as_deref()) else {
            return Vec::new();
        };
        let linker = RustCompanionCrate::toml_path(linker);
        let compiler = format!("CC_{}", triple.replace('-', "_"));
        vec![
            String::from("--config"),
            format!("target.{triple}.linker={linker}"),
            String::from("--config"),
            format!("env.{compiler}={linker}"),
        ]
    }

    fn vendor_arguments(&self) -> Vec<String> {
        let source = Self::VENDOR_SOURCE;
        let directory = RustCompanionCrate::toml_path(self.vendor);
        let arguments: Option<Vec<String>> = self.declaration.has_dependencies().then(|| {
            vec![
                String::from("--config"),
                format!("source.crates-io.replace-with=\"{source}\""),
                String::from("--config"),
                format!("source.{source}.directory={directory}"),
            ]
        });
        arguments.unwrap_or_default()
    }

    fn text(path: &Path) -> Result<String, Failure> {
        path.to_str().map(String::from).ok_or_else(|| {
            native_tool_failure(
                Self::FAILURE_CODE,
                "A Rust build path is not valid Unicode.",
            )
        })
    }
}

impl RustCrateBuild<'_> {
    const RELEASE: &'static str = "release";
    const EXECUTABLE_SUFFIXES: [&'static str; 2] = ["", ".exe"];
    const WINDOWS: &'static str = "windows";
    const APPLE: &'static str = "apple";

    const fn crate_types(linkage: Linkage) -> &'static [&'static str] {
        match linkage {
            Linkage::Static => &["staticlib", "rlib"],
            Linkage::Shared | Linkage::Web(_) => &["cdylib", "rlib"],
        }
    }

    fn library_file(linkage: Linkage, target_triple: Option<&str>, windows: bool) -> String {
        let library = RustCompanionCrate::LIBRARY;
        let apple = target_triple.map_or(cfg!(target_os = "macos"), |triple| {
            triple.contains(Self::APPLE)
        });
        match (linkage, windows, apple) {
            (Linkage::Static, true, _) => format!("{library}.lib"),
            (Linkage::Static, false, _) => format!("lib{library}.a"),
            (_, true, _) => format!("{library}.dll"),
            (_, false, true) => format!("lib{library}.dylib"),
            (_, false, false) => format!("lib{library}.so"),
        }
    }

    fn copy(from: &Path, to: &Path) -> Result<(), Failure> {
        let parent = to.parent().unwrap_or_else(|| Path::new("."));
        native_io(std::fs::create_dir_all(parent), parent)?;
        native_io(std::fs::copy(from, to), from).map(|_| ())
    }
}
