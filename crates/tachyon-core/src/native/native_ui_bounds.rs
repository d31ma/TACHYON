pub(super) struct NativeUiBounds {
    pub(super) asset_bytes: u64,
    pub(super) total_bytes: u64,
}

impl NativeUiBounds {
    pub(super) fn web() -> Self {
        Self {
            asset_bytes: 16 * 1024 * 1024,
            total_bytes: 64 * 1024 * 1024,
        }
    }

    pub(super) fn artifact() -> Self {
        Self {
            asset_bytes: 64 * 1024 * 1024,
            total_bytes: 256 * 1024 * 1024,
        }
    }
}

impl Default for NativeUiBounds {
    fn default() -> Self {
        Self::web()
    }
}
