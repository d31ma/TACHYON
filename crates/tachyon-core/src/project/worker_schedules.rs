use super::{CapturedFile, CapturedSources, ScheduledWorker, SnapshotContext};
use crate::Workers;
use crate::failure::{diagnostic, source_span};
use crate::schedule::Schedule;
use std::collections::BTreeMap;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct WorkerSchedules;

impl WorkerSchedules {
    const WORKER_CONFIG: &'static str = ".tachyonrc";
    const WORKER_ROOT: &'static str = "server/workers/";

    pub(super) fn discover(
        context: &SnapshotContext<'_>,
        captured: &CapturedSources,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Vec<ScheduledWorker> {
        let configured = Self::configured(&captured.configs, diagnostics);
        let declared = Self::declared(&captured.server, diagnostics);
        Self::report_twice_scheduled(&configured, &declared, diagnostics);
        let mut schedules: BTreeMap<String, Schedule> = configured
            .iter()
            .map(|(relative, every_seconds)| {
                let schedule = Schedule {
                    every_seconds,
                    initial_seconds: every_seconds,
                };
                (relative.clone(), schedule)
            })
            .collect();
        schedules.extend(declared);
        schedules
            .into_iter()
            .filter_map(|(relative, schedule)| {
                Self::worker(context, &captured.server, relative, schedule, diagnostics)
            })
            .collect()
    }

    fn configured(configs: &[CapturedFile], diagnostics: &mut Vec<Diagnostic>) -> Workers {
        let worker_config = configs
            .iter()
            .find(|file| file.relative == Path::new(Self::WORKER_CONFIG))
            .map(|file| file.bytes.as_slice());
        Workers::from_captured(worker_config).unwrap_or_else(|failure| {
            diagnostics.extend_from_slice(failure.diagnostics());
            Workers::default()
        })
    }

    fn declared(
        server_files: &[CapturedFile],
        diagnostics: &mut Vec<Diagnostic>,
    ) -> BTreeMap<String, Schedule> {
        server_files
            .iter()
            .filter_map(|file| Self::declared_in(file, diagnostics))
            .collect()
    }

    fn declared_in(
        file: &CapturedFile,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<(String, Schedule)> {
        let portable = file.portable();
        let worker_bytes = portable
            .starts_with(Self::WORKER_ROOT)
            .then_some(file.bytes.as_slice())?;
        let source = std::str::from_utf8(worker_bytes).ok()?;
        let schedule = crate::schedule::declared(&portable, source)
            .map_err(|failure| diagnostics.extend_from_slice(failure.diagnostics()))
            .ok()
            .flatten()?;
        Some((portable, schedule))
    }

    fn report_twice_scheduled(
        configured: &Workers,
        declared: &BTreeMap<String, Schedule>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        configured
            .iter()
            .filter(|(relative, _)| declared.contains_key(*relative))
            .for_each(|(relative, _)| diagnostics.push(Self::scheduled_twice(relative)));
    }

    fn scheduled_twice(relative: &str) -> Diagnostic {
        diagnostic(
            1502,
            format!("Worker '{relative}' declares Schedule and a .tachyonrc interval; keep one"),
            Some(String::from(
                "Delete the .tachyonrc entry or the Schedule marker.",
            )),
            source_span(relative, 0, relative.len()),
        )
    }

    fn worker(
        context: &SnapshotContext<'_>,
        server_files: &[CapturedFile],
        relative: String,
        schedule: Schedule,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<ScheduledWorker> {
        let Some(file) = server_files
            .iter()
            .find(|file| file.relative == Path::new(&relative))
        else {
            diagnostics.push(Self::missing_worker(&relative));
            return None;
        };
        let source = context
            .discover_handler(&file.relative, file.bytes.clone())
            .map_err(|failure| diagnostics.extend_from_slice(failure.diagnostics()))
            .ok()?;
        Some(ScheduledWorker {
            relative,
            every_seconds: schedule.every_seconds,
            initial_seconds: schedule.initial_seconds,
            source,
        })
    }

    fn missing_worker(relative: &str) -> Diagnostic {
        diagnostic(
            2001,
            format!("Cannot inspect handler source '{relative}': source does not exist"),
            None,
            source_span(relative, 0, relative.len()),
        )
    }
}
