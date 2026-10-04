use crate::Failure;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeMenuAction {
    pub(super) service: String,
    pub(super) method: String,
    #[serde(default)]
    pub(super) event: Option<String>,
}

impl NativeMenuAction {
    const MAX_NAME: usize = 64;

    pub(super) fn validate(&self) -> Result<(), Failure> {
        let valid = [
            Self::type_name(&self.service),
            Self::method_name(&self.method),
            self.event.as_deref().is_none_or(Self::type_name),
        ] == [true; 3];
        valid.then_some(()).ok_or_else(|| super::config::config_failure(
            "Native menu action needs a service type name, a snake_case method name and an optional event type name, each 1 to 64 ASCII characters.",
        ))
    }

    fn type_name(name: &str) -> bool {
        [
            name.len() <= Self::MAX_NAME,
            name.starts_with(|first: char| first.is_ascii_uppercase()),
            name.bytes().all(|byte| byte.is_ascii_alphanumeric()),
        ] == [true; 3]
    }

    fn method_name(name: &str) -> bool {
        [
            name.len() <= Self::MAX_NAME,
            name.starts_with(Self::method_start),
            name.bytes().all(Self::method_byte),
        ] == [true; 3]
    }

    fn method_start(first: char) -> bool {
        [first.is_ascii_lowercase(), first == '_'].contains(&true)
    }

    fn method_byte(byte: u8) -> bool {
        [
            byte.is_ascii_lowercase(),
            byte.is_ascii_digit(),
            byte == b'_',
        ]
        .contains(&true)
    }
}
