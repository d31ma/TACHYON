#![allow(clippy::needless_bitwise_bool)]

use cap_std::fs::Dir;
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RuleSwitch {
    tac: bool,
    yon: bool,
}

impl RuleSwitch {
    pub(crate) const ON_BY_DEFAULT: bool = true;
    pub(crate) const NONE: Self = Self {
        tac: false,
        yon: false,
    };
    const MANIFEST: &'static str = "client/manifest.json";
    const APPLICATION: &'static str = "server/application.json";

    pub(crate) fn cohesion(project: &Dir) -> Self {
        Self::read(project, "cohesion")
    }

    pub(crate) fn duplication(project: &Dir) -> Self {
        Self::read(project, "duplication")
    }

    #[cfg(test)]
    pub(crate) const fn every_realm() -> Self {
        Self {
            tac: true,
            yon: true,
        }
    }

    pub(crate) fn any_realm(self) -> bool {
        self.tac | self.yon
    }

    pub(crate) fn covers(self, path: &Path) -> bool {
        let client = path.starts_with("client");
        (client & self.tac) | (!client & self.yon)
    }

    fn read(project: &Dir, rule: &str) -> Self {
        Self::read_or(project, rule, Self::ON_BY_DEFAULT)
    }

    pub(crate) fn read_or(project: &Dir, rule: &str, default: bool) -> Self {
        Self {
            tac: Self::switch(
                project,
                Self::MANIFEST,
                &format!("/tac/{rule}/check"),
                default,
            ),
            yon: Self::switch(
                project,
                Self::APPLICATION,
                &format!("/{rule}/check"),
                default,
            ),
        }
    }

    pub(crate) fn suggestions(project: &Dir, rule: &str) -> Self {
        Self {
            tac: Self::switch(
                project,
                Self::MANIFEST,
                &format!("/tac/{rule}/suggest"),
                true,
            ),
            yon: Self::switch(
                project,
                Self::APPLICATION,
                &format!("/{rule}/suggest"),
                true,
            ),
        }
    }

    fn switch(project: &Dir, file: &str, pointer: &str, default: bool) -> bool {
        Self::stored_switch(project, file, pointer).unwrap_or(default)
    }

    fn stored_switch(project: &Dir, file: &str, pointer: &str) -> Option<bool> {
        let bytes = project.read(file).ok()?;
        let document: Value = serde_json::from_slice(&bytes).ok()?;
        document.pointer(pointer)?.as_bool()
    }
}
