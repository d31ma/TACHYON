use super::Toolchain;
use super::target_probe::TargetProbe;
use crate::{AppTarget, Choice};

#[derive(Clone, Copy, Debug)]
pub(super) struct TargetToolchain {
    pub(super) target: AppTarget,
    pub(super) command: &'static str,
    pub(super) probe: TargetProbe,
    pub(super) install: &'static str,
}

impl TargetToolchain {
    pub(super) fn is_for(&self, target: AppTarget) -> bool {
        self.target == target
    }

    pub(super) fn toolchain(&self) -> Toolchain {
        Toolchain {
            requirement: format!("{} apps", self.target.flag_name()),
            command: String::from(self.command),
            state: self.probe.state(),
            install: String::from(self.install),
            required: false,
        }
    }
}
