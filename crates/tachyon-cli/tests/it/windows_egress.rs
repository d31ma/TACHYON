use std::process::Command;

pub struct WindowsEgress;

impl WindowsEgress {
    const ENFORCEMENT: &'static str = "YON_EGRESS_ENFORCEMENT";
    const PROXY: &'static str = "proxy";

    pub fn proxied_where_unconfinable(command: &mut Command) -> &mut Command {
        if cfg!(windows) {
            command.env(Self::ENFORCEMENT, Self::PROXY);
        }
        command
    }
}
