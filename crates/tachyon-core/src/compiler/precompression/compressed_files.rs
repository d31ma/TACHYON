use super::{CompressibleFile, CompressionTotals, ContentEncoding, EncodingJob, PreviousOutput};
use crate::Failure;
use crate::compiler::{BuildStage, PortablePath};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

pub(crate) struct CompressedFiles {
    variants: BTreeMap<PathBuf, Vec<u8>>,
}

impl CompressedFiles {
    const MAX_WORKERS: usize = 4;

    pub(crate) fn create(
        files: &[(PathBuf, Vec<u8>)],
        previous_output: &Path,
        enabled: bool,
    ) -> Result<Self, Failure> {
        let jobs = Self::enabled_jobs(files, enabled);
        let previous = PreviousOutput::new(previous_output);
        let variants = BuildStage::output_io(Self::encode_all(&jobs, &previous), previous_output)?;
        Ok(Self {
            variants: variants.into_iter().collect(),
        })
    }

    pub(crate) fn write(&self, stage: &Path) -> io::Result<()> {
        self.variants
            .iter()
            .try_for_each(|(path, bytes)| fs::write(stage.join(path), bytes))
    }

    pub(crate) fn sent_bytes(&self, portable_path: &str) -> Option<u64> {
        let file = Path::new(portable_path);
        ContentEncoding::ALL
            .iter()
            .find_map(|encoding| self.variants.get(&encoding.variant_path(file)))
            .map(|bytes| u64::try_from(bytes.len()).unwrap_or(u64::MAX))
    }

    pub(crate) fn totals(&self, files: &[(PathBuf, Vec<u8>)]) -> CompressionTotals {
        files
            .iter()
            .filter_map(|(path, bytes)| {
                let sent = self.sent_bytes(&PortablePath::of(path))?;
                Some((u64::try_from(bytes.len()).unwrap_or(u64::MAX), sent))
            })
            .fold(CompressionTotals::default(), |totals, (raw, sent)| {
                CompressionTotals {
                    files: totals.files + 1,
                    raw_bytes: totals.raw_bytes + raw,
                    sent_bytes: totals.sent_bytes + sent,
                }
            })
    }

    fn enabled_jobs(files: &[(PathBuf, Vec<u8>)], enabled: bool) -> Vec<EncodingJob<'_>> {
        if enabled {
            return Self::jobs(files);
        }
        Vec::new()
    }

    fn jobs(files: &[(PathBuf, Vec<u8>)]) -> Vec<EncodingJob<'_>> {
        let mut jobs: Vec<EncodingJob<'_>> = files
            .iter()
            .filter(|(path, bytes)| CompressibleFile::accepts(path, bytes))
            .flat_map(|(path, bytes)| {
                ContentEncoding::ALL.map(|encoding| EncodingJob {
                    file: path,
                    bytes,
                    encoding,
                })
            })
            .collect();
        jobs.sort_by_key(|job| std::cmp::Reverse(job.bytes.len()));
        jobs
    }

    fn encode_all(
        jobs: &[EncodingJob<'_>],
        previous: &PreviousOutput,
    ) -> io::Result<Vec<(PathBuf, Vec<u8>)>> {
        let next = AtomicUsize::new(0);
        let workers = Self::worker_count(jobs.len());
        thread::scope(|scope| {
            let running: Vec<_> = (0..workers)
                .map(|_| scope.spawn(|| Self::encode_next(jobs, previous, &next)))
                .collect();
            running
                .into_iter()
                .map(|worker| worker.join().unwrap_or_else(|_| Err(Self::worker_failed())))
                .try_fold(Vec::new(), |mut all, encoded| {
                    all.extend(encoded?);
                    Ok(all)
                })
        })
    }

    fn encode_next(
        jobs: &[EncodingJob<'_>],
        previous: &PreviousOutput,
        next: &AtomicUsize,
    ) -> io::Result<Vec<(PathBuf, Vec<u8>)>> {
        let mut encoded = Vec::new();
        while let Some(job) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
            encoded.extend(job.run(previous)?);
        }
        Ok(encoded)
    }

    fn worker_count(job_count: usize) -> usize {
        let available = thread::available_parallelism().map_or(1, usize::from);
        available.min(Self::MAX_WORKERS).min(job_count)
    }

    fn worker_failed() -> io::Error {
        io::Error::other("A compression worker stopped unexpectedly")
    }
}
