use super::ToolchainState;
use serde::Serialize;
use std::fmt::Write as _;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Toolchain {
    pub requirement: String,
    pub command: String,
    pub state: ToolchainState,
    pub install: String,
    pub required: bool,
}

impl Toolchain {
    pub(super) fn is_ready(&self) -> bool {
        self.state.is_ready()
    }

    pub(super) fn blocks_the_project(&self) -> bool {
        self.required_but_not_ready()
    }

    pub(super) fn write_line(
        &self,
        text: &mut String,
        requirement_width: usize,
        command_width: usize,
    ) {
        let mark = self.mark();
        let detail = self.detail();
        let _ = writeln!(
            text,
            "{mark:<8}  {:<requirement_width$}  {:<command_width$}  {detail}",
            self.requirement, self.command
        );
        text.push_str(&self.install_line());
    }

    fn required_but_not_ready(&self) -> bool {
        self.required && !self.state.is_ready()
    }

    fn mark(&self) -> &'static str {
        Self::mark_for(self.is_optional_gap(), &self.state)
    }

    fn is_optional_gap(&self) -> bool {
        !self.required && !self.state.is_ready()
    }

    fn mark_for(optional_gap: bool, state: &ToolchainState) -> &'static str {
        match (optional_gap, state) {
            (true, _) => "info",
            (false, ToolchainState::Ready(_)) => "ready",
            (false, ToolchainState::Incomplete(_)) => "partial",
            (false, ToolchainState::Missing) => "missing",
        }
    }

    fn detail(&self) -> String {
        match &self.state {
            ToolchainState::Ready(version) => version.clone(),
            ToolchainState::Incomplete(reason) => reason.clone(),
            ToolchainState::Missing => String::from("not on PATH"),
        }
    }

    fn install_line(&self) -> String {
        if self.state.is_ready() {
            return String::new();
        }
        format!("        {}\n", self.install)
    }
}
