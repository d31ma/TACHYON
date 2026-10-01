use super::LuaSource;
use crate::behavior::{LuaBehaviorMethod, LuaBehaviorPlan};
use std::fmt::Write as _;
use std::path::Path;

pub(crate) struct LuaBehaviorEdits<'p> {
    plan: &'p LuaBehaviorPlan,
}

impl<'p> LuaBehaviorEdits<'p> {
    const RUNTIME: &'static str = include_str!("lua_behaviors.lua");
    const RESOURCE: &'static str = "Lua classes do not support {} yet (ADR 0085): it is a resource, like DataSource, RedisTemplate and relay. Write this class in another Yon language";
    const METHOD_END: &'static str = "\nend";
    const INNER_ENDS: [&'static str; 2] = ["end) ", "end "];

    pub(crate) fn new(plan: &'p LuaBehaviorPlan) -> Self {
        Self { plan }
    }

    pub(crate) fn program(&self, path: &Path, authored: &str) -> Result<String, String> {
        self.refuse_resources()?;
        let masked = crate::annotation_layout::masked_source(path, authored)
            .map_err(|failure| failure.to_string())?;
        Ok(format!(
            "{}\n{}\n{}",
            Self::applied(authored, self.edits(&masked)?),
            Self::RUNTIME,
            self.registration()
        ))
    }

    fn edits(&self, masked: &str) -> Result<Vec<(usize, String)>, String> {
        let mut edits = Vec::new();
        for method in &self.plan.methods {
            edits.extend(Self::method_edits(masked, method)?);
        }
        Ok(edits)
    }

    fn refuse_resources(&self) -> Result<(), String> {
        self.plan
            .methods
            .iter()
            .find_map(|method| method.resource)
            .map_or(Ok(()), |name| Err(Self::RESOURCE.replace("{}", name)))
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
        let end = masked[close..]
            .find(Self::METHOD_END)
            .map(|found| close + found + 1)
            .ok_or_else(missing)?;
        Ok([
            (close + 1, Self::opening(method)),
            (
                end,
                String::from(Self::INNER_ENDS[usize::from(Self::needs_inner_end(method))]),
            ),
        ])
    }

    fn needs_inner_end(method: &LuaBehaviorMethod) -> bool {
        method.validated.is_some() || method.mapping.is_some()
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

    fn depth_change(byte: u8) -> i64 {
        i64::from(byte == b'(') - i64::from(byte == b')')
    }

    fn opening(method: &LuaBehaviorMethod) -> String {
        let arguments = method.parameters.join(", ");
        if let Some(policy) = &method.mapping {
            return Self::mapping(policy, &arguments);
        }
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
            "{names}Behaviors.register({}, \"yon\", {:?})\n",
            LuaSource::long_string(&self.plan.records),
            self.plan.module_id
        )
    }
}
