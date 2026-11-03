use super::NativePackageFolder;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::{Path, PathBuf};

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub(crate) struct NativeModuleVendor;

impl NativeModuleVendor {
    pub(crate) const CODE: u16 = 1368;
    pub(crate) const HELP: &'static str = "Tachyon builds Go offline and never downloads modules. Third-party modules that server/packages/go.mod requires must be vendored: run `go mod vendor` in server/packages and commit go.sum and the vendor/ folder with vendor/modules.txt. Run it again after every change to a require line (ADR 0095).";
    const FOLDER: &'static str = "vendor";
    const REQUIRE: &'static str = "require";
    const NEEDED_FILES: [&'static str; 2] = ["go.sum", "vendor/modules.txt"];

    pub(crate) fn contains(path: &Path) -> bool {
        path.starts_with(Path::new(NativePackageFolder::ROOT).join(Self::FOLDER))
    }

    pub(crate) fn requires_modules(module_file: &str) -> bool {
        NativePackageFolder::directives(module_file).contains(&Self::REQUIRE)
    }

    pub(crate) fn check<'a>(
        files: impl Iterator<Item = (&'a Path, &'a [u8])>,
    ) -> Result<(), Failure> {
        let root = Path::new(NativePackageFolder::ROOT);
        let native = files
            .filter(|(path, _)| path.starts_with(root))
            .collect::<Vec<_>>();
        Self::missing_when_vendored(&native)
            .map_or(Ok(()), |missing| Err(Self::missing_files(&missing)))
    }

    pub(crate) fn failure(path: &str, message: String) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(String::from(Self::HELP)),
            source_span(path, 0, 0),
        ))
    }

    fn missing_when_vendored(native: &[(&Path, &[u8])]) -> Option<Vec<PathBuf>> {
        let root = Path::new(NativePackageFolder::ROOT);
        let missing = Self::NEEDED_FILES
            .iter()
            .map(|needed| root.join(needed))
            .filter(|needed| !native.iter().any(|(path, _)| path == needed))
            .collect::<Vec<_>>();
        (Self::vendored(native) && !missing.is_empty()).then_some(missing)
    }

    fn vendored(native: &[(&Path, &[u8])]) -> bool {
        let module_file =
            Path::new(NativePackageFolder::ROOT).join(NativePackageFolder::MODULE_FILE);
        native
            .iter()
            .find(|(path, _)| *path == module_file)
            .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
            .is_some_and(Self::requires_modules)
    }

    fn missing_files(missing: &[PathBuf]) -> Failure {
        let names = missing
            .iter()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>();
        Self::failure(
            &format!(
                "{}/{}",
                NativePackageFolder::ROOT,
                NativePackageFolder::MODULE_FILE
            ),
            format!(
                "server/packages/go.mod requires third-party modules, but {} is missing. Run `go mod vendor` in server/packages.",
                names.join(" and ")
            ),
        )
    }
}
