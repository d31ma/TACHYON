use super::ToolCommand;
use crate::Failure;

pub trait ToolRunner {
    fn run(&mut self, command: &ToolCommand) -> Result<(), Failure>;
}
