use super::LuaSource;
use crate::behavior::{LuaBehaviorMethod, LuaBehaviorPlan};
use std::fmt::Write as _;
use std::path::Path;

pub(crate) struct LuaBehaviorEdits<'p> {
    plan: &'p LuaBehaviorPlan,
    environment: &'static str,
}

impl<'p> LuaBehaviorEdits<'p> {
    pub(crate) const RUNTIME: &'static str = include_str!("lua_behaviors.lua");
    const BLOCK_OPENERS: [&'static str; 3] = ["function", "if", "do"];
    const BLOCK_END: &'static str = "end";
    const INNER_ENDS: [&'static str; 2] = ["end) ", "end "];

    pub(crate) fn new(plan: &'p LuaBehaviorPlan) -> Self {
        Self {
            plan,
            environment: "yon",
        }
    }

    pub(crate) fn tac(plan: &'p LuaBehaviorPlan) -> Self {
        Self {
            plan,
            environment: "tac",
        }
    }

    pub(crate) fn program(&self, path: &Path, authored: &str) -> Result<String, String> {
        Ok(format!(
            "{}\n{}\n{}",
            self.wrapped(path, authored)?,
            Self::RUNTIME,
            self.registration()
        ))
    }

    pub(crate) fn tac_program(&self, path: &Path, authored: &str) -> Result<String, String> {
        Ok(format!(
            "{}\n{}",
            self.wrapped(path, authored)?,
            self.registration()
        ))
    }

    fn wrapped(&self, path: &Path, authored: &str) -> Result<String, String> {
        let masked = crate::annotation_layout::masked_source(path, authored)
            .map_err(|failure| failure.to_string())?;
        Ok(Self::applied(authored, self.edits(&masked)?))
    }

    fn edits(&self, masked: &str) -> Result<Vec<(usize, String)>, String> {
        let mut edits = Vec::new();
        for method in &self.plan.methods {
            edits.extend(Self::method_edits(masked, method)?);
        }
        Ok(edits)
    }

    fn method_edits(
        masked: &str,
        method: &LuaBehaviorMethod,
    ) -> Result<[(usize, String); 2], String> {
        let missing = || {
            format!(
                "Cannot find the Lua method {}:{}",
                method.owner, method.name
            )
        };
        let signature = format!("function {}:{}(", method.owner, method.name);
        let start = Self::line_start_of(masked, &signature).ok_or_else(missing)?;
        let open = start + signature.len() - 1;
        let close = Self::closing(masked, open).ok_or_else(missing)?;
        let end = Self::function_end(masked, close).ok_or_else(missing)?;
        Ok([
            (close + 1, Self::opening(method)),
            (
                end,
                String::from(Self::INNER_ENDS[usize::from(Self::needs_inner_end(method))]),
            ),
        ])
    }

    fn needs_inner_end(method: &LuaBehaviorMethod) -> bool {
        [
            method.validated.is_some(),
            method.mapping.is_some(),
            method.gateway.is_some(),
        ]
        .contains(&true)
    }

    fn line_start_of(masked: &str, signature: &str) -> Option<usize> {
        masked
            .match_indices(signature)
            .map(|(at, _)| at)
            .find(|at| Self::starts_line(masked, *at))
    }

    fn starts_line(masked: &str, at: usize) -> bool {
        masked[..at].ends_with('\n') || at == 0
    }

    fn closing(masked: &str, open: usize) -> Option<usize> {
        masked[open..]
            .bytes()
            .scan(0_i64, |depth, byte| {
                *depth += Self::depth_change(byte);
                Some(*depth)
            })
            .position(|depth| depth == 0)
            .map(|offset| open + offset)
    }

    fn function_end(masked: &str, after: usize) -> Option<usize> {
        Self::words(masked, after)
            .scan(1_i64, |depth, (at, word)| {
                *depth += Self::block_change(word);
                Some((at, *depth))
            })
            .find(|(_, depth)| *depth == 0)
            .map(|(at, _)| at)
    }

    fn block_change(word: &str) -> i64 {
        i64::from(Self::BLOCK_OPENERS.contains(&word)) - i64::from(word == Self::BLOCK_END)
    }

    fn words(masked: &str, after: usize) -> impl Iterator<Item = (usize, &str)> {
        masked[after..]
            .split_inclusive(|character: char| !Self::is_word_character(character))
            .scan(after, |at, piece| {
                let start = *at;
                *at += piece.len();
                Some((
                    start,
                    piece.trim_end_matches(|character: char| !Self::is_word_character(character)),
                ))
            })
    }

    const fn is_word_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || character == '_'
    }

    fn depth_change(byte: u8) -> i64 {
        i64::from(byte == b'(') - i64::from(byte == b')')
    }

    fn opening(method: &LuaBehaviorMethod) -> String {
        let arguments = method.parameters.join(", ");
        method
            .mapping
            .as_ref()
            .map(|policy| Self::mapping(policy, &arguments))
            .or_else(|| {
                method
                    .gateway
                    .as_ref()
                    .map(|key| Self::gateway(key, &arguments))
            })
            .unwrap_or_else(|| Self::invocation(method, &arguments))
    }

    fn gateway(key: &str, arguments: &str) -> String {
        format!(
            " do return BehaviorBoundary.gateway({}, table.pack({arguments}))",
            LuaSource::long_string(key)
        )
    }

    fn invocation(method: &LuaBehaviorMethod, arguments: &str) -> String {
        method.validated.as_ref().map_or_else(
            || {
                format!(
                    " return self.behaviors:invoke(self, {:?}, {:?}, table.pack({arguments}), function({arguments})",
                    method.owner, method.name
                )
            },
            |(index, schema)| Self::validation(method, *index, schema),
        )
    }

    fn mapping(policy: &serde_json::Value, arguments: &str) -> String {
        let plan = LuaSource::long_string(&policy.to_string());
        let mapped =
            format!("BehaviorBoundary.map_value(YonJson.decode({plan}), {{ {arguments} }})");
        let result = policy.get("result").and_then(serde_json::Value::as_str);
        let read = result.map_or(mapped.clone(), |name| {
            format!("YonModels.read({{ model = {name} }}, {mapped})")
        });
        format!(" do return {read}")
    }

    fn validation(method: &LuaBehaviorMethod, index: usize, schema: &str) -> String {
        let name = method.parameters.get(index).cloned().unwrap_or_default();
        format!(
            " {name} = BehaviorValidation.apply(YonJson.decode({}), {name}); do",
            LuaSource::long_string(schema)
        )
    }

    fn applied(authored: &str, mut edits: Vec<(usize, String)>) -> String {
        edits.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
        edits
            .into_iter()
            .fold(authored.to_owned(), |mut text, (at, insert)| {
                text.insert_str(at, &insert);
                text
            })
    }

    fn registration(&self) -> String {
        let names = self
            .plan
            .types
            .iter()
            .fold(String::new(), |mut lines, name| {
                let _ = writeln!(lines, "{name}.__name = {name:?}");
                lines
            });
        format!(
            "{names}Behaviors.register({}, {:?}, {:?})\n",
            LuaSource::long_string(&self.plan.records),
            self.environment,
            self.plan.module_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::LuaBehaviorEdits;
    use crate::behavior::LuaBehaviorMethod;

    fn method_end(source: &str) -> usize {
        let method = LuaBehaviorMethod {
            owner: String::from("Catalog"),
            name: String::from("list"),
            parameters: vec![String::from("limit")],
            mapping: None,
            validated: None,
            gateway: None,
        };
        LuaBehaviorEdits::method_edits(source, &method)
            .unwrap_or_else(|message| panic!("{message}"))[1]
            .0
    }

    #[test]
    fn the_method_ends_at_the_end_that_closes_its_own_function() {
        for source in [
            "function Catalog:list(limit)\nendpoint = limit\n  return endpoint\nend\n",
            "function Catalog:list(limit)\n  if limit then\n    return limit\nend\n  return 0\nend\n",
            "function Catalog:list(limit)\n  for index = 1, limit do\n    local read = function() return index end\n  end\n  while false do end\n  repeat limit = limit - 1 until limit < 0\n  return limit\nend\n",
        ] {
            assert_eq!(
                method_end(source),
                source.rfind("end").unwrap_or(0),
                "{source}"
            );
        }
    }
}
