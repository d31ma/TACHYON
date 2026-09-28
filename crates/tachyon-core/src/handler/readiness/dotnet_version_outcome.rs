use super::RuntimeProbeState;

#[derive(Clone, Copy)]
pub(super) enum DotnetVersionOutcome {
    Finished(RuntimeProbeState),
    SdkMajor(u32),
}
