use crate::yon::plan::Plan;
use std::path::Path;

pub(super) struct LuaResources;

impl LuaResources {
    const TYPES: [&'static str; 2] = ["DataSource", "RedisTemplate"];
    const RELAY: &'static str = "relay(";
    const RELAY_NAME: &'static str = "relay";
    const MESSAGE: &'static str = "Lua classes do not support {} yet (ADR 0085): it is a resource, like DataSource, RedisTemplate, gateway clients and relay. Write the class that needs it in another Yon language";

    pub(super) fn refuse(plan: &Plan, source: &Path, contents: &str) -> Result<(), String> {
        let code = crate::annotation_layout::masked_source(source, contents)
            .map_err(|failure| failure.to_string())?;
        Self::named_type(plan)
            .or_else(|| Self::relay(&code))
            .map_or(Ok(()), |name| Err(Self::MESSAGE.replace("{}", name)))
    }

    fn named_type(plan: &Plan) -> Option<&'static str> {
        Self::TYPES
            .into_iter()
            .find(|name| Self::provides(plan, name))
    }

    fn provides(plan: &Plan, name: &str) -> bool {
        plan.providers.iter().any(|provider| provider.name == name)
    }

    fn relay(code: &str) -> Option<&'static str> {
        code.match_indices(Self::RELAY)
            .any(|(at, _)| Self::starts_word(code, at))
            .then_some(Self::RELAY_NAME)
    }

    fn starts_word(code: &str, at: usize) -> bool {
        !code[..at].ends_with(|character: char| {
            [character.is_alphanumeric(), "_.:".contains(character)].contains(&true)
        })
    }
}
