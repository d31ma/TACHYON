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
    const RESERVED: [&'static str; 12] = [
        "command:q",
        "command:w",
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
    const WINDOWS_RESERVED: [&'static str; 1] = ["command:y"];

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

    pub(super) fn desktop_identity(&self) -> String {
        let modifiers = self
            .modifiers
            .iter()
            .map(|modifier| Self::desktop_modifier(modifier))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        format!("{}:{}", modifiers.join("+"), self.key)
    }

    pub(super) fn require_desktop_available(
        &self,
        target: tachyon_contracts::NativeTarget,
    ) -> Result<(), Failure> {
        let identity = self.desktop_identity();
        let reserved = Self::reserved_on(target).contains(&identity.as_str());
        (!reserved).then_some(()).ok_or_else(|| super::config::config_failure(&format!(
            "Native menu shortcut '{}' becomes {} on {target:?}, where control is Ctrl like command; that key belongs to a standard File, Edit or Window command, so choose another key.",
            self.identity(),
            self.accelerator_text(),
        )))
    }

    fn desktop_modifier(modifier: &str) -> &str {
        match modifier {
            "control" => "command",
            other => other,
        }
    }

    fn reserved_on(target: tachyon_contracts::NativeTarget) -> Vec<&'static str> {
        let extra: &[&'static str] = if target == tachyon_contracts::NativeTarget::Windows {
            &Self::WINDOWS_RESERVED
        } else {
            &[]
        };
        Self::RESERVED.iter().chain(extra).copied().collect()
    }

    fn accelerator_text(&self) -> String {
        let identity = self.desktop_identity();
        let (modifiers, key) = identity.split_once(':').unwrap_or_default();
        let prefix = modifiers
            .split('+')
            .map(Self::accelerator_modifier)
            .collect::<String>();
        format!("{prefix}{}", key.to_ascii_uppercase())
    }

    fn accelerator_modifier(modifier: &str) -> &'static str {
        match modifier {
            "command" => "Ctrl+",
            "option" => "Alt+",
            "shift" => "Shift+",
            _ => "",
        }
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
            .ok_or_else(|| super::config::config_failure("Native menu shortcut conflicts with a standard application, File, Edit or Window command."))
    }
}
