use super::native_menu_action::NativeMenuAction;
use super::native_menu_shortcut::NativeMenuShortcut;
use crate::Failure;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeMenuEntry {
    pub(super) id: String,
    pub(super) label: String,
    #[serde(default)]
    pub(super) route: Option<String>,
    #[serde(default)]
    pub(super) action: Option<NativeMenuAction>,
    #[serde(default)]
    pub(super) shortcut: Option<NativeMenuShortcut>,
}

impl NativeMenuEntry {
    pub(super) fn validate(&self) -> Result<(), Failure> {
        self.require_id()?;
        self.require_label()?;
        self.require_destination()?;
        self.shortcut
            .as_ref()
            .map_or(Ok(()), NativeMenuShortcut::validate)
    }

    fn require_id(&self) -> Result<(), Failure> {
        let valid = (1..=64).contains(&self.id.len()) && self.id.bytes().all(Self::id_byte);
        valid.then_some(()).ok_or_else(|| {
            super::config::config_failure(
                "Native menu id must contain 1 to 64 ASCII letters, digits, '.', '_' or '-'.",
            )
        })
    }

    fn id_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || b"._-".contains(&byte)
    }

    fn require_label(&self) -> Result<(), Failure> {
        let valid = !self.label.trim().is_empty()
            && self.label.len() <= 128
            && !self.label.chars().any(char::is_control);
        valid.then_some(()).ok_or_else(|| {
            super::config::config_failure(
                "Native menu label must contain 1 to 128 UTF-8 bytes without control characters.",
            )
        })
    }

    fn require_destination(&self) -> Result<(), Failure> {
        match (&self.route, &self.action) {
            (Some(route), None) => Self::require_route(route),
            (None, Some(action)) => action.validate(),
            _ => Err(super::config::config_failure(&format!(
                "Native menu '{}' needs exactly one of route or action.",
                self.id
            ))),
        }
    }

    fn require_route(route: &str) -> Result<(), Failure> {
        let valid = super::config::valid_route(route)
            && !route.contains(['%', '?', '#', ':'])
            && !route.chars().any(char::is_control);
        valid.then_some(()).ok_or_else(|| super::config::config_failure(
            "Native menu route must be a canonical local path of at most 2048 bytes without query, fragment or encoded characters.",
        ))
    }
}
