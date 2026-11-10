use crate::sorted_json::SortedJson;
use std::ffi::{OsStr, OsString};

pub struct EgressLaunch;

impl EgressLaunch {
    const SWITCH: &'static str = "__yon-egress";
    #[cfg(target_os = "linux")]
    const END: &'static str = "--";
    const REFUSED_STATUS: i32 = 126;
    #[cfg(target_os = "linux")]
    const MISSING_STATUS: i32 = 127;

    pub fn run_if_requested() {
        let mut arguments = std::env::args_os().skip(1);
        let requested = arguments.next().as_deref() == Some(OsStr::new(Self::SWITCH));
        Self::launch_when(requested, arguments);
    }

    fn launch_when(requested: bool, arguments: impl Iterator<Item = OsString>) {
        if requested {
            Self::launch(arguments);
        }
    }

    #[cfg(target_os = "linux")]
    pub(super) fn wrapper(ports: &[u16]) -> Result<super::CommandWrapper, String> {
        super::ConnectRules::check()?;
        super::SocketFilter::program()?;
        let launcher = Self::launcher()?;
        let mut prefix: Vec<OsString> = vec![Self::SWITCH.into()];
        prefix.extend(ports.iter().map(|port| OsString::from(port.to_string())));
        prefix.push(Self::END.into());
        Ok(super::CommandWrapper::new(launcher, prefix))
    }

    #[cfg(target_os = "linux")]
    fn launcher() -> Result<std::path::PathBuf, String> {
        let current = std::env::current_exe().map_err(|error| error.to_string())?;
        let name = Self::launcher_name();
        if current.file_name() == Some(name) {
            return Ok(current);
        }
        current
            .ancestors()
            .skip(1)
            .take(2)
            .map(|directory| directory.join(name))
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| String::from("the ty program that confines handlers was not found"))
    }

    #[cfg(target_os = "linux")]
    fn launcher_name() -> &'static OsStr {
        OsStr::new("ty")
    }

    #[cfg(target_os = "linux")]
    fn launch(mut arguments: impl Iterator<Item = OsString>) -> ! {
        use std::os::unix::process::CommandExt;
        let ports: Vec<u16> = arguments
            .by_ref()
            .take_while(|argument| argument != Self::END)
            .filter_map(|argument| argument.to_str()?.parse().ok())
            .collect();
        let program = arguments
            .next()
            .unwrap_or_else(|| Self::refuse("no program was named"));
        Self::confine(&ports).unwrap_or_else(|reason| Self::refuse(&reason));
        let error = std::process::Command::new(program).args(arguments).exec();
        Self::report(&error.to_string());
        std::process::exit(Self::MISSING_STATUS);
    }

    #[cfg(target_os = "linux")]
    fn confine(ports: &[u16]) -> Result<(), String> {
        let program = super::SocketFilter::program()?;
        super::ConnectRules::restrict(ports)?;
        super::SocketFilter::apply(&program)
    }

    #[cfg(not(target_os = "linux"))]
    fn launch(_arguments: impl Iterator<Item = OsString>) -> ! {
        Self::refuse("this operating system has no egress launcher");
    }

    fn refuse(reason: &str) -> ! {
        Self::report(reason);
        std::process::exit(Self::REFUSED_STATUS);
    }

    fn report(reason: &str) {
        eprintln!(
            "{}",
            SortedJson::text(
                &serde_json::json!({ "event": "handler.egress_refused", "reason": reason })
            )
        );
    }
}
