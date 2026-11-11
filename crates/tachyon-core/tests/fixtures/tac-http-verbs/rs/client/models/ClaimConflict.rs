#[derive(Clone, Debug, Default)]
pub struct ClaimConflict {
    pub reason: String,
    pub holder: String,
}
