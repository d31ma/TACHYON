pub(super) struct NativeExternalLink;

impl NativeExternalLink {
    pub(super) const C_HELPER: &'static str = include_str!("native_external_link.h");
}
