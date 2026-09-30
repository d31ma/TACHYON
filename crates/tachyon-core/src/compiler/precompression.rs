mod compressed_files;
mod compressible_file;
mod compression_cache;
mod compression_totals;
mod content_encoding;
mod encoding_job;
mod previous_output;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod precompression_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod compression_cache_tests;

pub(crate) use compressed_files::CompressedFiles;
use compressible_file::CompressibleFile;
use compression_cache::CompressionCache;
pub use compression_totals::CompressionTotals;
use content_encoding::ContentEncoding;
use encoding_job::EncodingJob;
use previous_output::PreviousOutput;
