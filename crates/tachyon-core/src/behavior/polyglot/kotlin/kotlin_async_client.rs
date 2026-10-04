use super::super::{Method, Type};
use serde_json::Value;

pub(super) struct KotlinAsyncClient<'a> {
    owner: &'a Type,
    method: &'a Method,
}

impl<'a> KotlinAsyncClient<'a> {
    const TAC: &'static str = "tac";
    const REFUSED: [&'static str; 5] = [
        "retryable",
        "concurrencyLimit",
        "transactional",
        "valid",
        "pageable",
    ];
    const RAW_ANSWER: &'static str = "Any?";
    const NO_ANSWER: &'static str = "Unit";
    const OPTIONAL: char = '?';
    const LIST_STARTS: [&'static str; 4] = ["List<", "MutableList<", "Collection<", "Iterable<"];
    const PLAIN: [&'static str; 6] = ["String", "Int", "Long", "Double", "Float", "Boolean"];

    pub(super) fn applies(method: &Method, environment: &str) -> bool {
        [
            environment == Self::TAC,
            method.asynchronous,
            super::declared_exchange(method),
        ] == [true; 3]
    }

    pub(super) fn body(owner: &'a Type, method: &'a Method) -> Result<String, String> {
        let client = Self { owner, method };
        client.validate()?;
        let (prefix, call) = client.call()?;
        Ok(format!("{prefix}{}", client.answer(&call)))
    }

    fn validate(&self) -> Result<(), String> {
        if !super::has_behaviors_context(self.owner) {
            return Err(
                "Behavior services require an explicit Behaviors.Context behaviors field"
                    .to_owned(),
            );
        }
        Self::REFUSED
            .iter()
            .find(|key| self.method.metadata.contains_key(**key))
            .map_or(Ok(()), |key| {
                Err(format!(
                    "Kotlin suspend HTTPClient method {} cannot use {key}; call it from a service method that retries or limits instead",
                    self.method.name
                ))
            })
    }

    fn call(&self) -> Result<(String, String), String> {
        let exchange = super::exchange_policy(self.method.metadata.get("httpExchange"))?;
        let arguments = format!(
            "arrayOf<Any?>({})",
            super::parameter_names(self.method).join(", ")
        );
        let owner = super::quote(&self.owner.name);
        if let Some(mutation) = super::mutation_metadata(self.method) {
            return self.mutation(mutation, &exchange, &arguments);
        }
        let cached = self.method.metadata.get("cacheable");
        let policy = super::cache_policy_when_cached(self.owner, cached)?;
        let selected = cached.map_or_else(|| Ok("intArrayOf()".to_owned()), super::indices)?;
        Ok((
            String::new(),
            format!(
                "this.behaviors.invokeAwait({owner}, {}, {arguments}, {selected}, {policy}, {exchange})",
                super::quote(&self.method.name)
            ),
        ))
    }

    fn mutation(
        &self,
        mutation: &Value,
        exchange: &str,
        arguments: &str,
    ) -> Result<(String, String), String> {
        let target = mutation
            .get("target")
            .ok_or("Cache mutation requires a resolved target")?;
        let selected = super::indices(
            target
                .get("cacheable")
                .ok_or("Cache mutation requires Cached target")?,
        )?;
        let mapped = super::mutation_arguments(self.method, mutation)?;
        let mut prefix = format!("val __ty_target = arrayOf<Any?>({})\n", mapped.join(", "));
        prefix.push_str(&super::target_page(target)?);
        let cache = self
            .owner
            .cache
            .as_ref()
            .ok_or("Cache mutation requires Cache")?;
        let put = self.method.metadata.contains_key("cachePut");
        Ok((
            prefix,
            format!(
                "this.behaviors.mutateAwait({}, {}, __ty_target, {selected}, {}, {put}, {exchange}, {arguments})",
                super::quote(&self.owner.name),
                super::string(target, "method")?,
                super::policy(cache)?
            ),
        ))
    }

    fn answer(&self, call: &str) -> String {
        let declared = self.method.return_type.trim();
        if [declared.is_empty(), declared == Self::NO_ANSWER].contains(&true) {
            return format!("{call}\n");
        }
        if declared == Self::RAW_ANSWER {
            return format!("return {call}\n");
        }
        let required = declared.trim_end_matches(Self::OPTIONAL);
        format!(
            "return Behaviors.answer({call}, \"{}\", {}) {{ tacRead<{declared}>(it) }}\n",
            Self::shape(required),
            declared.ends_with(Self::OPTIONAL)
        )
    }

    fn shape(required: &str) -> &'static str {
        if Self::LIST_STARTS
            .iter()
            .any(|start| required.starts_with(start))
        {
            return "list";
        }
        if Self::PLAIN.contains(&required) {
            return "plain";
        }
        "object"
    }
}
