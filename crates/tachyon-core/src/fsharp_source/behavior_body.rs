use crate::behavior::PlannedMethod;
use serde_json::Value;

pub(crate) struct BehaviorBody<'a> {
    method: &'a PlannedMethod,
    receiver: &'a str,
}

impl<'a> BehaviorBody<'a> {
    const INVOKED: [&'static str; 4] = ["cacheable", "httpExchange", "cacheEvict", "cachePut"];
    const ABSTRACT_ONLY: &'static str = "An abstract F# member can only hold Query, Convert, Gateway or a generated WebCall; write a member with a body instead";
    const NULL_ANSWER: &'static str =
        "System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse(\"null\"))";

    pub(crate) fn new(method: &'a PlannedMethod, receiver: &'a str) -> Self {
        Self { method, receiver }
    }

    pub(crate) fn expression(&self, original: Option<&str>) -> Result<String, String> {
        self.written()?.map_or_else(|| self.composed(original), Ok)
    }

    fn composed(&self, original: Option<&str>) -> Result<String, String> {
        let prefix = self.prefix()?;
        let body = self.wrapped(original)?;
        Ok(format!("{prefix}{body}"))
    }

    fn written(&self) -> Result<Option<String>, String> {
        let metadata = &self.method.metadata;
        let gateway = metadata.get("gateway").map(|policy| self.gateway(policy));
        let query = metadata.get("query").map(|query| self.query(query));
        let mapping = metadata
            .get("mapping")
            .map(|policy| self.mapping(policy))
            .transpose()?;
        Ok(gateway.or(query).or(mapping))
    }

    fn query(&self, query: &Value) -> String {
        format!(
            "{}.Query(BehaviorJSON.Parse({}), {})",
            self.receiver,
            Self::quote(&query.to_string()),
            self.arguments()
        )
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
        if let Some(result) = policy.get("result").and_then(Value::as_str) {
            let plan = Self::quote(&policy.to_string());
            return Ok(if policy["realm"] == "tac" {
                format!(
                    "TacMappingModels.read{result}(BehaviorContext.MapValue(BehaviorJSON.Parse({plan}), TacMappingModels.write(box {value})))"
                )
            } else {
                format!(
                    "System.Text.Json.JsonSerializer.Deserialize<{result}>(BehaviorContext.MapValue(BehaviorJSON.Parse({plan}), BehaviorJSON.Parse(System.Text.Json.JsonSerializer.Serialize({value}, YonModels.Options))).ToString(), YonModels.Options)"
                )
            });
        }
        Ok(format!(
            "BehaviorContext.MapValue(BehaviorJSON.Parse({}), BehaviorJSON.From({value}))",
            Self::quote(&policy.to_string())
        ))
    }

    fn prefix(&self) -> Result<String, String> {
        let metadata = &self.method.metadata;
        let validation = metadata
            .get("valid")
            .map(|policy| self.validation(policy))
            .transpose()?;
        let page = metadata
            .get("pageable")
            .map(|policy| self.page(policy))
            .transpose()?;
        Ok(format!(
            "{}{}",
            validation.unwrap_or_default(),
            page.unwrap_or_default()
        ))
    }

    fn validation(&self, policy: &Value) -> Result<String, String> {
        let name = self.parameter(policy, "Check")?;
        Ok(format!(
            "let {name} = Behaviors.CheckSchema(BehaviorJSON.Parse({}), {name}) in ",
            Self::quote(&policy["schema"].to_string())
        ))
    }

    fn page(&self, policy: &Value) -> Result<String, String> {
        let name = self.parameter(policy, "PageDefaults")?;
        Ok(format!(
            "let {name} = BehaviorPage.Normalize({name}, BehaviorJSON.Parse({})) in ",
            Self::quote(&policy.to_string())
        ))
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
        let generated = self.generated().then(|| String::from(Self::NULL_ANSWER));
        let body = generated
            .or_else(|| original.map(str::to_owned))
            .ok_or(Self::ABSTRACT_ONLY)?;
        Ok(self.around(&body))
    }

    fn around(&self, body: &str) -> String {
        match [self.only_validates(), self.invoked()] {
            [true, _] => body.to_owned(),
            [false, true] => self.invoke(body),
            [false, false] => format!("{}.Paginate(fun () -> {body})", self.receiver),
        }
    }

    fn generated(&self) -> bool {
        self.method
            .metadata
            .get("httpExchange")
            .is_some_and(|exchange| exchange.get("generated").is_some())
    }

    fn only_validates(&self) -> bool {
        let metadata = &self.method.metadata;
        [metadata.len() == 1, metadata.contains_key("valid")] == [true, true]
    }

    fn invoked(&self) -> bool {
        let marked = Self::INVOKED
            .iter()
            .any(|key| self.method.metadata.contains_key(*key));
        [self.generated(), self.method.asynchronous, marked].contains(&true)
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
