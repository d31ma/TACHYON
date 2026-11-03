use crate::Failure;
use crate::tac_cpp::{CppClass, CppFailure, CppMember, CppMemberKind};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct CppYonPolicies;

impl CppYonPolicies {
    const FIELD: &'static str = "behaviors";
    const SCOPE: &'static str = " ::tachyon::yon::YonStorage tachyonStorage(behaviors);";
    const CONTEXT_TYPES: [&'static str; 3] = [
        "BehaviorContext",
        "tachyon::BehaviorContext",
        "::tachyon::BehaviorContext",
    ];
    const STORED: [&'static str; 2] = ["cache", "transactional"];
    const NEEDS_CONTEXT: &'static str = "A Yon C++ class whose methods carry [[Cached]], [[UpdateCache]], [[ClearCache]] or [[AllOrNothing]] keeps its adapters in a `BehaviorContext behaviors;` field and opens it from a Setup, for example in an [[OnStart]] method: `behaviors = Behaviors::create<CatalogSetup>(options);`";

    pub(crate) fn scope(
        path: &Path,
        class: &CppClass,
        behaviors: &BTreeMap<String, Value>,
    ) -> Result<&'static str, Failure> {
        let held = class.members.iter().any(Self::holds_context);
        let stored = behaviors
            .iter()
            .find(|(_, policy)| Self::uses_storage(policy))
            .map(|(name, _)| name.as_str());
        let missing = stored.filter(|_| !held);
        missing.map_or(Ok(()), |name| Err(Self::needs_context(path, class, name)))?;
        Ok(["", Self::SCOPE][usize::from(held)])
    }

    fn uses_storage(policy: &Value) -> bool {
        Self::STORED.iter().any(|key| !policy[*key].is_null())
    }

    fn holds_context(member: &CppMember) -> bool {
        [
            member.is_instance(CppMemberKind::Field),
            member.name == Self::FIELD,
            Self::CONTEXT_TYPES.contains(&member.plain_type().as_str()),
        ] == [true; 3]
    }

    fn needs_context(path: &Path, class: &CppClass, name: &str) -> Failure {
        let at = class
            .members
            .iter()
            .find(|member| member.name == name)
            .map_or(class.name_at, |member| member.name_at);
        CppFailure::unreadable(path, at, Self::NEEDS_CONTEXT)
    }
}
