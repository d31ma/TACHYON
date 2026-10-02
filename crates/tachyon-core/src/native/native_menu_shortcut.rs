use crate::Failure;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeMenuShortcut {
    pub(super) key: String,
    pub(super) modifiers: Vec<String>,
}

impl NativeMenuShortcut {
    const MODIFIERS: [&'static str; 4] = ["command", "control", "option", "shift"];
    const RESERVED: [&'static str; 11] = [
        "command:q",
        "command:h",
        "command+option:h",
        "command:z",
        "command+shift:z",
        "command:x",
        "command:c",
        "command:v",
        "command:a",
        "command:m",
        "command+control:f",
    ];

    pub(super) fn validate(&self) -> Result<(), Failure> {
        self.require_key()?;
        self.require_modifiers()?;
        self.require_available()
    }

    pub(super) fn identity(&self) -> String {
        let mut modifiers = self.modifiers.clone();
        modifiers.sort();
        format!("{}:{}", modifiers.join("+"), self.key)
    }

    fn require_key(&self) -> Result<(), Failure> {
        let valid = self.key.len() == 1 && self.key.bytes().all(Self::key_byte);
        valid.then_some(()).ok_or_else(|| {
            super::config::config_failure(
                "Native menu shortcut key must be one lowercase ASCII letter or digit.",
            )
        })
    }

    fn key_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() && !byte.is_ascii_uppercase()
    }

    fn require_modifiers(&self) -> Result<(), Failure> {
        let unique = self
            .modifiers
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let valid = self.modifiers.iter().any(|modifier| modifier == "command")
            && unique.len() == self.modifiers.len()
            && self
                .modifiers
                .iter()
                .all(|modifier| Self::MODIFIERS.contains(&modifier.as_str()));
        valid.then_some(()).ok_or_else(|| super::config::config_failure(
            "Native menu shortcuts require command and distinct known command/control/option/shift modifiers.",
        ))
    }

    fn require_available(&self) -> Result<(), Failure> {
        (!Self::RESERVED.contains(&self.identity().as_str())).then_some(())
            .ok_or_else(|| super::config::config_failure("Native menu shortcut conflicts with a standard application, Edit or Window command."))
    }
}
