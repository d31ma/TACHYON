use crate::behavior::PlannedMethod;
use serde_json::Value;
use std::fmt::Write as _;

pub(crate) struct BehaviorBody<'a> {
    method: &'a PlannedMethod,
    receiver: &'a str,
}

impl<'a> BehaviorBody<'a> {
    const INVOKED: [&'static str; 4] = ["cacheable", "httpExchange", "cacheEvict", "cachePut"];
    const NULL_ANSWER: &'static str =
        "System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse(\"null\"))";

    pub(crate) fn new(method: &'a PlannedMethod, receiver: &'a str) -> Self {
        Self { method, receiver }
    }

    pub(crate) fn expression(&self, original: Option<&str>) -> Result<String, String> {
        if let Some(written) = self.written()? {
            return Ok(written);
        }
        let prefix = self.prefix()?;
        let body = self.wrapped(original)?;
        Ok(format!("{prefix}{body}"))
    }

    fn written(&self) -> Result<Option<String>, String> {
        let metadata = &self.method.metadata;
        if let Some(policy) = metadata.get("gateway") {
            return Ok(Some(self.gateway(policy)));
        }
        if let Some(query) = metadata.get("query") {
            return Ok(Some(format!(
                "{}.Query(BehaviorJSON.Parse({}), {})",
                self.receiver,
                Self::quote(&query.to_string()),
                self.arguments()
            )));
        }
        if let Some(policy) = metadata.get("mapping") {
            return self.mapping(policy).map(Some);
        }
        Ok(None)
    }

    fn gateway(&self, policy: &Value) -> String {
        let key = policy["key"].as_str().unwrap_or_default();
        format!(
            "Behaviors.GatewayResult(YonGateway.Call({}, Behaviors.GatewayArguments([| {} |])))",
            Self::quote(key),
            self.method.parameters.join("; ")
        )
    }

    fn mapping(&self, policy: &Value) -> Result<String, String> {
        let value = self
            .method
            .mapped_argument
            .and_then(|index| self.method.parameters.get(index))
            .ok_or("Convert reads an argument the method does not declare")?;
        Ok(format!(
            "{}.MapValue(BehaviorJSON.Parse({}), BehaviorJSON.From({value}))",
            self.receiver,
            Self::quote(&policy.to_string())
        ))
    }

    fn prefix(&self) -> Result<String, String> {
        let mut prefix = String::new();
        if let Some(policy) = self.method.metadata.get("valid") {
            let name = self.parameter(policy, "Check")?;
            let _ = write!(
                prefix,
                "let {name} = Behaviors.ValidateSchema(BehaviorJSON.Parse({}), {name}) in ",
                Self::quote(&policy["schema"].to_string())
            );
        }
        if let Some(policy) = self.method.metadata.get("pageable") {
            let name = self.parameter(policy, "PageDefaults")?;
            let _ = write!(
                prefix,
                "let {name} = BehaviorPage.Normalize({name}, BehaviorJSON.Parse({})) in ",
                Self::quote(&policy.to_string())
            );
        }
        Ok(prefix)
    }

    fn parameter(&self, policy: &Value, annotation: &str) -> Result<&str, String> {
        policy["argument"]
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| self.method.parameters.get(index))
            .map(String::as_str)
            .ok_or_else(|| format!("{annotation} argument is out of range"))
    }

    fn wrapped(&self, original: Option<&str>) -> Result<String, String> {
        let metadata = &self.method.metadata;
        let generated = metadata
            .get("httpExchange")
            .is_some_and(|exchange| exchange.get("generated").is_some());
        if generated {
            return Ok(self.invoke(Self::NULL_ANSWER));
        }
        let original = original.ok_or(
            "An abstract F# member can only hold Query, Convert, Gateway or a generated WebCall; write a member with a body instead",
        )?;
        if metadata.len() == 1 && metadata.contains_key("valid") {
            return Ok(original.to_owned());
        }
        let invoked =
            self.method.asynchronous || Self::INVOKED.iter().any(|key| metadata.contains_key(*key));
        if invoked {
            return Ok(self.invoke(original));
        }
        Ok(format!("{}.Paginate(fun () -> {original})", self.receiver))
    }

    fn invoke(&self, body: &str) -> String {
        format!(
            "{}.Invoke({}, {}, {}, fun () -> {body})",
            self.receiver,
            Self::quote(&self.method.owner),
            Self::quote(&self.method.name),
            self.arguments()
        )
    }

    fn arguments(&self) -> String {
        let values: Vec<String> = self
            .method
            .parameters
            .iter()
            .map(|name| format!("BehaviorJSON.From({name})"))
            .collect();
        format!("[| {} |]", values.join("; "))
    }

    fn quote(value: &str) -> String {
        Value::String(value.to_owned()).to_string()
    }
}
