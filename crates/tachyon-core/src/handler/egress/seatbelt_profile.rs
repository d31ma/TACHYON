use super::CommandWrapper;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Stdio;

pub(super) struct SeatbeltProfile;

impl SeatbeltProfile {
    const PROGRAM: &'static str = "/usr/bin/sandbox-exec";
    const PROFILE_SWITCH: &'static str = "-p";
    const CHECK_PROGRAM: &'static str = "/usr/bin/true";

    pub(super) async fn wrapper(ports: &[u16]) -> Result<CommandWrapper, String> {
        let profile = Self::text(ports);
        Self::check(&profile).await?;
        Ok(CommandWrapper::new(
            PathBuf::from(Self::PROGRAM),
            vec![Self::PROFILE_SWITCH.into(), profile.into()],
        ))
    }

    pub(super) fn text(ports: &[u16]) -> String {
        let mut profile = String::from("(version 1)(allow default)(deny network*)");
        for port in ports {
            let _ = write!(
                profile,
                "(allow network-outbound (remote ip \"localhost:{port}\"))"
            );
        }
        profile
    }

    async fn check(profile: &str) -> Result<(), String> {
        let status = tokio::process::Command::new(Self::PROGRAM)
            .args([Self::PROFILE_SWITCH, profile, Self::CHECK_PROGRAM])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|error| format!("{} cannot start ({error})", Self::PROGRAM))?;
        if !status.success() {
            return Err(format!("{} refused the network profile", Self::PROGRAM));
        }
        Ok(())
    }
}
