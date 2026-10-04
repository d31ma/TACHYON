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
        target: Option<tachyon_contracts::NativeTarget>,
    ) -> Result<(), Failure> {
        services
            .iter()
            .try_for_each(|(_, service)| Self::validate(snapshot_root, service, target))
    }

    #[cfg(test)]
    pub(super) fn check_native_source(
        snapshot_root: &Path,
        path: &Path,
        source: &str,
    ) -> Result<(), Failure> {
        Self::check_source(snapshot_root, path, source, false)
    }

    pub(super) fn check_source(
        snapshot_root: &Path,
        path: &Path,
        source: &str,
        browser: bool,
    ) -> Result<(), Failure> {
        let assembled = crate::annotation_modules::assemble(snapshot_root, path, source, "tac")?;
        let assembled_source = assembled.as_deref().unwrap_or(source);
        let events = crate::events::extract(path, assembled_source, "tac")?;
        let lowered_source = events
            .as_ref()
            .map_or(assembled_source, |module| module.source.as_str());
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        crate::model_sources::ModelSources::client_metadata(snapshot_root, extension)?.lowered(
            path,
            lowered_source,
            |text| {
                Self::lower(snapshot_root, path, text, browser)
                    .map_err(|failure| Self::located_failure(path, failure, text))
            },
        )?;
        Ok(())
    }

    fn validate(
        snapshot_root: &Path,
        service: &ServiceDefinition,
        target: Option<tachyon_contracts::NativeTarget>,
    ) -> Result<(), Failure> {
        Self::validate_native_companions(snapshot_root, service.native_companions(), target)?;
        service.go_path().map_or(Ok(()), |path| {
            Self::validate_source_file(snapshot_root, path, target)
        })
    }

    fn validate_native_companions(
        snapshot_root: &Path,
        companions: &[(NativeCompanion, PathBuf)],
        target: Option<tachyon_contracts::NativeTarget>,
    ) -> Result<(), Failure> {
        companions
            .iter()
            .try_for_each(|(_, path)| Self::validate_source_file(snapshot_root, path, target))
    }

    fn validate_source_file(
        snapshot_root: &Path,
        path: &Path,
        target: Option<tachyon_contracts::NativeTarget>,
    ) -> Result<(), Failure> {
        let relative = path.strip_prefix(snapshot_root).unwrap_or(path);
        fs::read_to_string(path).map_or(Ok(()), |source| {
            Self::validate_source(snapshot_root, relative, &source, target)
        })
    }

    fn validate_source(
        snapshot_root: &Path,
        relative: &Path,
        source: &str,
        target: Option<tachyon_contracts::NativeTarget>,
    ) -> Result<(), Failure> {
        let roles = crate::behavior::ServiceRoles::read(relative, source)?;
        crate::behavior::NativeApplicationLifetime::target(relative, roles.application, target)?;
        Self::check_source(snapshot_root, relative, &roles.source, target.is_none())
    }

    fn lower(
        snapshot_root: &Path,
        path: &Path,
        source: &str,
        browser: bool,
    ) -> Result<Option<String>, Failure> {
        if browser {
            super::captured_browser_behavior::CapturedBrowserBehavior::lower(
                snapshot_root,
                path,
                source,
                &std::collections::BTreeMap::new(),
            )
        } else {
            crate::behavior::lower(path, source, "tac")
        }
    }

    fn located_failure(path: &Path, failure: Failure, lowered_source: &str) -> Failure {
        if path.extension().is_some_and(|extension| extension == "go") {
            return crate::go_source::GoFile::locate(&failure, lowered_source);
        }
        failure
    }
}
