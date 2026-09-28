use super::{InstallStep, NeededTool, PlanKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallPlan {
    pub tool: NeededTool,
    pub kind: PlanKind,
    pub steps: Vec<InstallStep>,
    pub lines: Vec<String>,
}

impl InstallPlan {
    #[must_use]
    pub fn automatic(tool: NeededTool, steps: Vec<InstallStep>, lines: Vec<String>) -> Self {
        Self {
            tool,
            kind: PlanKind::Automatic,
            steps,
            lines,
        }
    }

    #[must_use]
    pub fn advice(tool: NeededTool, kind: PlanKind, lines: Vec<String>) -> Self {
        Self {
            tool,
            kind,
            steps: Vec::new(),
            lines,
        }
    }

    #[must_use]
    pub fn needs_admin(tool: NeededTool, lines: Vec<String>) -> Self {
        Self {
            tool,
            kind: PlanKind::NeedsAdmin,
            steps: Vec::new(),
            lines,
        }
    }

    #[must_use]
    pub fn manual(tool: NeededTool, lines: Vec<String>) -> Self {
        Self {
            tool,
            kind: PlanKind::Manual,
            steps: Vec::new(),
            lines,
        }
    }
}
