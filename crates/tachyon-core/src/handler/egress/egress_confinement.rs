use super::{CommandWrapper, EgressEnforcement};
use crate::Failure;
use crate::failure::diagnostic;
use tokio::process::Command;

#[derive(Clone, Debug, Default)]
pub(crate) struct EgressConfinement {
    wrapper: Option<CommandWrapper>,
}

impl EgressConfinement {
    pub(crate) async fn prepare(
        enforcement: EgressEnforcement,
        required: bool,
        ports: &[u16],
    ) -> Result<Self, Failure> {
        if enforcement == EgressEnforcement::Proxy {
            Self::report_unenforced(&format!("{}=proxy", EgressEnforcement::NAME));
            return Ok(Self::default());
        }
        Self::settle(Self::platform(ports).await, required)
    }

    pub(super) fn settle(
        platform: Result<CommandWrapper, String>,
        required: bool,
    ) -> Result<Self, Failure> {
        match platform {
            Ok(wrapper) => Ok(Self {
                wrapper: Some(wrapper),
            }),
            Err(reason) if required => Err(Self::unavailable(&reason)),
            Err(reason) => {
                Self::report_unenforced(&reason);
                Ok(Self::default())
            }
        }
    }

    pub(crate) fn apply(&self, command: &mut Command) {
        if let Some(wrapper) = &self.wrapper {
            wrapper.wrap(command);
        }
    }

    #[cfg(test)]
    pub(crate) const fn enforced(&self) -> bool {
        self.wrapper.is_some()
    }

    #[cfg(target_os = "macos")]
    async fn platform(ports: &[u16]) -> Result<CommandWrapper, String> {
        super::SeatbeltProfile::wrapper(ports).await
    }

    #[cfg(target_os = "linux")]
    #[expect(clippy::unused_async)]
    async fn platform(ports: &[u16]) -> Result<CommandWrapper, String> {
        super::EgressLaunch::wrapper(ports)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    #[expect(clippy::unused_async)]
    async fn platform(_ports: &[u16]) -> Result<CommandWrapper, String> {
        Err(String::from(
            "this operating system cannot limit a process's network access without an administrator",
        ))
    }

    fn report_unenforced(reason: &str) {
        eprintln!("{}", Self::unenforced_event(reason));
    }

    pub(super) fn unenforced_event(reason: &str) -> serde_json::Value {
        serde_json::json!({
            "event": "handler.egress_unenforced",
            "level": "warning",
            "reason": reason,
            "message": "Handler processes can reach the network without going through the outgoing request guard (ADR 0113).",
        })
    }

    fn unavailable(reason: &str) -> Failure {
        Failure::one(diagnostic(
            2113,
            format!("Tachyon cannot confine the network access of handler processes: {reason}."),
            Some(format!(
                "Run on macOS or on Linux 6.7 or newer with Landlock enabled. Where the platform already limits egress, set {}=proxy in the environment of ty (ADR 0113).",
                EgressEnforcement::NAME
            )),
            None,
        ))
    }
}
