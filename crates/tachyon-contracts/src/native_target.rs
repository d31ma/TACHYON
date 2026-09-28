use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NativeTarget {
    Linux,
    Macos,
    Windows,
    Android,
    Ios,
}
