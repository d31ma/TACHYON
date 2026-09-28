use super::super::rule_switch::RuleSwitch;
use super::design_problem::DesignProblem;
use super::design_rule::DesignRule;
use super::project_words::ProjectWords;
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::path::Path;

pub(crate) struct DesignSwitches {
    switches: [RuleSwitch; 6],
    words: ProjectWords,
}

impl DesignSwitches {
    pub(crate) fn read(project: &Dir) -> Self {
        Self {
            switches: DesignRule::ALL
                .map(|rule| RuleSwitch::read_or(project, rule.switch_name(), rule.on_by_default())),
            words: ProjectWords::read(project),
        }
    }

    pub(super) fn read_folder(root: &Path) -> Option<Self> {
        let project = Dir::open_ambient_dir(root, ambient_authority()).ok()?;
        Some(Self::read(&project))
    }

    pub(super) fn covers_path(&self, rule: DesignRule, path: &str) -> bool {
        self.switches[rule as usize].covers(Path::new(path))
    }

    #[cfg(test)]
    pub(super) fn every_realm(words: ProjectWords) -> Self {
        Self {
            switches: [RuleSwitch::every_realm(); 6],
            words,
        }
    }

    pub(super) fn covers(&self, problem: &DesignProblem) -> bool {
        self.covers_path(problem.rule, &problem.path)
    }

    pub(super) fn any_on(&self) -> bool {
        self.switches.iter().any(|switch| switch.any_realm())
    }

    pub(super) fn words(&self) -> &ProjectWords {
        &self.words
    }
}
