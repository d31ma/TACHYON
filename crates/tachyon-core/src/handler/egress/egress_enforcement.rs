use crate::Failure;
use crate::failure::diagnostic;
use std::ffi::OsStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum EgressEnforcement {
    #[default]
    Os,
    Proxy,
}

impl EgressEnforcement {
    pub(crate) const NAME: &'static str = "YON_EGRESS_ENFORCEMENT";
    const OS: &'static str = "os";
    const PROXY: &'static str = "proxy";

    pub(crate) fn from_environment() -> Result<Self, Failure> {
        Self::parse(std::env::var_os(Self::NAME).as_deref())
    }

    pub(crate) fn parse(value: Option<&OsStr>) -> Result<Self, Failure> {
        let Some(value) = value else {
            return Ok(Self::Os);
        };
        match value.to_str() {
            Some(Self::OS) => Ok(Self::Os),
            Some(Self::PROXY) => Ok(Self::Proxy),
            _ => Err(Self::invalid()),
        }
    }

    fn invalid() -> Failure {
        Failure::one(diagnostic(
            2113,
            format!(
                "{} must be '{}' or '{}'.",
                Self::NAME,
                Self::OS,
                Self::PROXY
            ),
            Some(String::from(
                "Leave it unset to let the operating system confine handler egress (ADR 0113).",
            )),
            None,
        ))
    }
}
