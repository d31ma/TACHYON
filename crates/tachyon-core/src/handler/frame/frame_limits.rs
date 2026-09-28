pub(in crate::handler) struct FrameLimits;

impl FrameLimits {
    pub(in crate::handler) const MAX_BYTES: usize = 16 * 1024 * 1024;
    pub(in crate::handler) const PREFIX_BYTES: usize = 4;
}
