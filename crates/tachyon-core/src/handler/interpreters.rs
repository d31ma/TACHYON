mod raw_configuration;
mod raw_worker;

use crate::Failure;
use crate::failure::{diagnostic, source_span};
use raw_configuration::RawConfiguration;
use raw_worker::RawWorker;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Workers {
    schedules: BTreeMap<String, u64>,
}

impl Workers {
    const CONFIG_PATH: &str = ".tachyonrc";
    const MAX_CONFIG_BYTES: u64 = 64 * 1_024;
    const MAX_WORKERS: usize = 64;
    const MIN_WORKER_SECONDS: u64 = 1;
    const MAX_WORKER_SECONDS: u64 = 86_400;

    pub fn discover(project_root: &Path) -> Result<Self, Failure> {
        let path = project_root.join(Self::CONFIG_PATH);
        Self::config_metadata(&path)?.map_or_else(
            || Ok(Self::default()),
            |metadata| Self::from_metadata(&path, &metadata),
        )
    }

    pub(crate) fn from_captured(bytes: Option<&[u8]>) -> Result<Self, Failure> {
        bytes.map_or_else(|| Ok(Self::default()), Self::from_bytes)
    }

    fn config_metadata(path: &Path) -> Result<Option<fs::Metadata>, Failure> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => Ok(Some(metadata)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(Self::config_failure(&format!("Cannot inspect: {error}"))),
        }
    }

    fn from_metadata(path: &Path, metadata: &fs::Metadata) -> Result<Self, Failure> {
        Self::check_regular_file(metadata)?;
        Self::check_size(metadata.len())?;
        let bytes = fs::read(path)
            .map_err(|error| Self::config_failure(&format!("Cannot read: {error}")))?;
        Self::from_bytes(&bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Failure> {
        Self::check_size(bytes.len() as u64)?;
        let source =
            std::str::from_utf8(bytes).map_err(|_| Self::config_failure("Must be valid UTF-8."))?;
        let raw: RawConfiguration = serde_json::from_str(source)
            .map_err(|error| Self::config_failure(&format!("Is not valid JSON: {error}")))?;
        Self::check_no_interpreters(&raw)?;
        Self::check_worker_count(&raw)?;
        let schedules = raw
            .workers
            .into_iter()
            .map(|(source_path, worker)| Self::checked_schedule(source_path, &worker))
            .collect::<Result<BTreeMap<_, _>, Failure>>()?;
        Ok(Self { schedules })
    }

    fn check_regular_file(metadata: &fs::Metadata) -> Result<(), Failure> {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(Self::config_failure(
                "Must be a regular, non-symlinked file.",
            ));
        }
        Ok(())
    }

    fn check_size(length: u64) -> Result<(), Failure> {
        if length > Self::MAX_CONFIG_BYTES {
            return Err(Self::config_failure("Exceeds the 64 KiB limit."));
        }
        Ok(())
    }

    fn check_no_interpreters(raw: &RawConfiguration) -> Result<(), Failure> {
        if !raw.interpreters.is_empty() {
            return Err(Self::config_failure(
                "The 'interpreters' field was removed. Call relay(...) for non-Yon execution.",
            ));
        }
        Ok(())
    }

    fn check_worker_count(raw: &RawConfiguration) -> Result<(), Failure> {
        if raw.workers.len() > Self::MAX_WORKERS {
            return Err(Self::config_failure("Declares more than 64 workers."));
        }
        Ok(())
    }

    fn checked_schedule(source_path: String, worker: &RawWorker) -> Result<(String, u64), Failure> {
        Self::check_worker_path(&source_path)?;
        Self::check_worker_seconds(&source_path, worker)?;
        Ok((source_path, worker.every_seconds))
    }

    fn check_worker_path(source_path: &str) -> Result<(), Failure> {
        if !source_path.starts_with("server/workers/") || source_path.contains("..") {
            return Err(Self::config_failure(&format!(
                "Worker '{source_path}' must be a path under server/workers/."
            )));
        }
        Ok(())
    }

    fn check_worker_seconds(source_path: &str, worker: &RawWorker) -> Result<(), Failure> {
        let allowed_seconds = Self::MIN_WORKER_SECONDS..=Self::MAX_WORKER_SECONDS;
        if !allowed_seconds.contains(&worker.every_seconds) {
            return Err(Self::config_failure(&format!(
                "Worker '{source_path}' must run every 1 to 86400 seconds."
            )));
        }
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, u64)> {
        self.schedules
            .iter()
            .map(|(path, seconds)| (path, *seconds))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.schedules.is_empty()
    }

    fn config_failure(message: &str) -> Failure {
        let config_path = Self::CONFIG_PATH;
        Failure::one(diagnostic(
            1502,
            format!("{config_path} is invalid. {message}"),
            Some(String::from(
                "Keep only bounded worker schedules here. Yon runs its eight owned languages; \
                 reach any other program with an explicit relay(command, request) call.",
            )),
            source_span(config_path, 0, config_path.len()),
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::Workers;
    use std::fs;

    #[test]
    fn a_missing_configuration_schedules_nothing() {
        let root = tempfile::tempdir().expect("root");
        assert!(Workers::discover(root.path()).expect("none").is_empty());
    }

    #[test]
    fn interpreter_registration_is_rejected_with_relay_guidance() {
        let root = tempfile::tempdir().expect("project");
        fs::write(
            root.path().join(".tachyonrc"),
            r#"{"interpreters":{"rb":["ruby"]}}"#,
        )
        .expect("configuration");
        let error = Workers::discover(root.path()).expect_err("removed interpreter field");
        let rendered = error.to_string();
        assert!(rendered.contains("TY1502"), "{rendered}");
        assert!(rendered.contains("relay("), "{rendered}");
    }

    #[test]
    fn workers_are_scheduled_with_bounded_intervals() {
        let root = tempfile::tempdir().expect("root");
        fs::write(
            root.path().join(".tachyonrc"),
            r#"{"workers":{"server/workers/beat.py":{"every_seconds":30}}}"#,
        )
        .expect("configuration");
        let workers = Workers::discover(root.path()).expect("workers");
        assert_eq!(
            workers.iter().collect::<Vec<_>>(),
            vec![(&String::from("server/workers/beat.py"), 30)]
        );

        for source in [
            r#"{"workers":{"beat.py":{"every_seconds":30}}}"#,
            r#"{"workers":{"server/workers/../x.py":{"every_seconds":30}}}"#,
            r#"{"workers":{"server/workers/beat.py":{"every_seconds":0}}}"#,
            r#"{"workers":{"server/workers/beat.py":{"every_seconds":86401}}}"#,
        ] {
            let root = tempfile::tempdir().expect("root");
            fs::write(root.path().join(".tachyonrc"), source).expect("configuration");
            assert!(Workers::discover(root.path()).is_err(), "{source}");
        }
    }

    #[test]
    fn malformed_configurations_fail_closed() {
        for source in [r#"{"unknown":true}"#, "not json"] {
            let root = tempfile::tempdir().expect("root");
            fs::write(root.path().join(".tachyonrc"), source).expect("configuration");
            let error = Workers::discover(root.path()).expect_err(source);
            assert!(error.to_string().contains("TY1502"), "{source}: {error}");
        }
    }
}
