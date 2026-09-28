use crate::Failure;
use crate::project::NativeCompanion;
use crate::template::{ServiceDefinition, ServiceRegistry};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct ServiceChecks;

impl ServiceChecks {
    pub(super) fn validate_all(
        snapshot_root: &Path,
        services: &ServiceRegistry,
    ) -> Result<(), Failure> {
        services
            .iter()
            .try_for_each(|(_, service)| Self::validate(snapshot_root, service))
    }

    pub(super) fn check_native_source(
        snapshot_root: &Path,
        path: &Path,
        source: &str,
    ) -> Result<(), Failure> {
        let assembled = crate::annotation_modules::assemble(snapshot_root, path, source, "tac")?;
        let assembled_source = assembled.as_deref().unwrap_or(source);
        let events = crate::events::extract(path, assembled_source, "tac")?;
        let lowered_source = events
            .as_ref()
            .map_or(assembled_source, |module| module.source.as_str());
        crate::behavior::lower(path, lowered_source, "tac")
            .map_err(|failure| Self::located_failure(path, failure, lowered_source))?;
        Ok(())
    }

    fn validate(snapshot_root: &Path, service: &ServiceDefinition) -> Result<(), Failure> {
        Self::validate_native_companions(snapshot_root, service.native_companions())?;
        service.go_path().map_or(Ok(()), |path| {
            Self::validate_source_file(snapshot_root, path)
        })
    }

    fn validate_native_companions(
        snapshot_root: &Path,
        companions: &[(NativeCompanion, PathBuf)],
    ) -> Result<(), Failure> {
        companions
            .iter()
            .try_for_each(|(_, path)| Self::validate_source_file(snapshot_root, path))
    }

    fn validate_source_file(snapshot_root: &Path, path: &Path) -> Result<(), Failure> {
        let relative = path.strip_prefix(snapshot_root).unwrap_or(path);
        fs::read_to_string(path).map_or(Ok(()), |source| {
            Self::validate_source(snapshot_root, relative, &source)
        })
    }

    fn validate_source(snapshot_root: &Path, relative: &Path, source: &str) -> Result<(), Failure> {
        let roles = crate::behavior::ServiceRoles::read(relative, source)?;
        Self::check_native_source(snapshot_root, relative, &roles.source)
    }

    fn located_failure(path: &Path, failure: Failure, lowered_source: &str) -> Failure {
        if path.extension().is_some_and(|extension| extension == "go") {
            return crate::go_source::GoFile::locate(&failure, lowered_source);
        }
        failure
    }
}
