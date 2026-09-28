#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompressionTotals {
    pub(crate) files: usize,
    pub(crate) raw_bytes: u64,
    pub(crate) sent_bytes: u64,
}

impl CompressionTotals {
    const BYTES_PER_KILOBYTE: u64 = 1024;
    const PERCENT: u64 = 100;

    #[must_use]
    pub const fn files(&self) -> usize {
        self.files
    }

    #[must_use]
    pub const fn raw_bytes(&self) -> u64 {
        self.raw_bytes
    }

    #[must_use]
    pub const fn sent_bytes(&self) -> u64 {
        self.sent_bytes
    }

    #[must_use]
    pub fn line(&self) -> Option<String> {
        (self.files > 0).then(|| {
            format!(
                "Compressed {} files: {} KB -> {} KB with Brotli ({}% smaller).",
                self.files,
                self.raw_bytes.div_ceil(Self::BYTES_PER_KILOBYTE),
                self.sent_bytes.div_ceil(Self::BYTES_PER_KILOBYTE),
                self.saved_percent()
            )
        })
    }

    fn saved_percent(&self) -> u64 {
        let saved = self.raw_bytes.saturating_sub(self.sent_bytes);
        (saved * Self::PERCENT)
            .checked_div(self.raw_bytes)
            .unwrap_or_default()
    }
}
