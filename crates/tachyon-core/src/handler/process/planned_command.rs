use super::RuntimeIdentity;
use tokio::process::Command;

pub(super) struct PlannedCommand {
    pub(super) command: Command,
    pub(super) runtime: RuntimeIdentity,
}
