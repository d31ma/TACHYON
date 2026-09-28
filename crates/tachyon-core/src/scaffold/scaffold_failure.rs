use crate::Failure;
use crate::failure::diagnostic;
use std::path::Path;

pub(crate) struct ScaffoldFailure;

impl ScaffoldFailure {
    const TARGET: u16 = 1401;
    const NOT_EMPTY: u16 = 1402;
    const NAME: u16 = 1403;

    pub(crate) fn target(message: String) -> Failure {
        Failure::one(diagnostic(Self::TARGET, message, None, None))
    }

    pub(crate) fn io(target: &Path, error: &std::io::Error) -> Failure {
        Failure::one(diagnostic(
            Self::TARGET,
            format!("Cannot initialize project '{}': {error}", target.display()),
            Some(String::from("Check target permissions and try again.")),
            None,
        ))
    }

    pub(crate) fn not_empty(target: &Path) -> Failure {
        Failure::one(diagnostic(
            Self::NOT_EMPTY,
            format!("Initialization target '{}' is not empty.", target.display()),
            Some(String::from(
                "Choose a new or empty directory; Tachyon never overwrites existing projects.",
            )),
            None,
        ))
    }

    pub(crate) fn existing_file(path: &Path) -> Failure {
        Failure::one(diagnostic(
            Self::NOT_EMPTY,
            format!("'{}' already exists.", path.display()),
            Some(String::from(
                "Tachyon never overwrites your files. Move or rename the file, then run the command again.",
            )),
            None,
        ))
    }

    pub(crate) fn name() -> Failure {
        Failure::one(diagnostic(
            Self::NAME,
            "Application name must contain 1 to 100 printable characters.",
            None,
            None,
        ))
    }

    pub(crate) fn checked<T>(result: std::io::Result<T>, target: &Path) -> Result<T, Failure> {
        result.map_err(|error| Self::io(target, &error))
    }
}
