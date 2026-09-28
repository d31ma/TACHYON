use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CountedComparison {
    Lt,
    Le,
    Gt,
    Ge,
}

impl CountedComparison {
    const ASCENDING: [Self; 2] = [Self::Lt, Self::Le];

    #[must_use]
    pub fn ascending(self) -> bool {
        Self::ASCENDING.contains(&self)
    }
}
