use super::{Method, Type};

pub(super) struct CsharpModelExchange;

impl CsharpModelExchange {
    pub(super) fn body(declared: &Type, method: &Method, prefix: &str) -> Option<String> {
        Self::model_body(declared, method, prefix)
    }

    fn model_body(declared: &Type, method: &Method, prefix: &str) -> Option<String> {
        let model = method.metadata.get("tacModelResult")?;
        let name = model.get("name")?.as_str()?;
        let models = model.get("models")?.to_string();
        let arguments = super::csharp::json_arguments(method);
        Some(format!(
            "\n{prefix}var __tacAnswer = await this.behaviors.Invoke({}, {}, new BehaviorJSON[] {{ {arguments} }}, () => System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse(\"null\")));\ntry {{ return TacModels.ReadResult<Models.{name}>(__tacAnswer.ToString(), {}, {}); }}\ncatch {{ throw new BehaviorError(\"HTTP_RESPONSE\"); }}\n",
            serde_json::json!(declared.name),
            serde_json::json!(method.name),
            serde_json::json!(name),
            serde_json::json!(models)
        ))
    }
}
