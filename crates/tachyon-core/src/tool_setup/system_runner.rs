use super::{ToolCommand, ToolRunner};
use crate::Failure;
use crate::failure::diagnostic;
use std::process::Command;

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemRunner;

impl SystemRunner {
    const CODE: u16 = 1409;

    fn command(tool: &ToolCommand) -> Command {
        let mut command = Command::new(&tool.program);
        command.args(&tool.arguments);
        if let Some(folder) = &tool.folder {
            command.current_dir(folder);
        }
        command
    }

    fn failure(tool: &ToolCommand, reason: &str) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            format!("'{}' did not finish: {reason}", tool.text()),
            Some(String::from(
                "Read the output above, fix the problem, and run the command again.",
            )),
            None,
        ))
    }

    fn checked(tool: &ToolCommand, success: bool) -> Result<(), Failure> {
        if success {
            return Ok(());
        }
        Err(Self::failure(tool, "it stopped with an error"))
    }
}

impl ToolRunner for SystemRunner {
    fn run(&mut self, tool: &ToolCommand) -> Result<(), Failure> {
        let status = Self::command(tool)
            .status()
            .map_err(|error| Self::failure(tool, &error.to_string()))?;
        Self::checked(tool, status.success())
    }
}
