use super::{CheckRule, FieldPath};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use tachyon_contracts::HandlerViolation;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ProblemEntry {
    path: FieldPath,
    rule: CheckRule,
}

impl ProblemEntry {
    pub(crate) fn read(violation: &HandlerViolation) -> Option<Self> {
        let path = FieldPath::parse(&violation.field)?;
        let rule = CheckRule::parse(&violation.rule)?;
        Some(Self { path, rule })
    }
}

impl Serialize for ProblemEntry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut entry = serializer.serialize_struct("ProblemEntry", 2)?;
        entry.serialize_field("field", &self.path.to_string())?;
        entry.serialize_field("rule", self.rule.name())?;
        entry.end()
    }
}
