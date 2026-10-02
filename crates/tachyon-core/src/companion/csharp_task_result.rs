use crate::route_types::ModelTable;

pub(crate) struct CsharpTaskResult;

impl CsharpTaskResult {
    const TASK: &'static str = "Task<";
    const QUALIFIED_TASK: &'static str = "System.Threading.Tasks.Task<";

    pub(crate) fn payload(kind: &str) -> Option<&str> {
        Self::argument(kind.trim())
    }

    pub(crate) fn model(kind: &str, models: &ModelTable) -> Result<Option<String>, String> {
        Self::require_task(kind)?;
        Self::registered(kind, models)
    }

    fn require_task(kind: &str) -> Result<(), String> {
        let malformed = Self::malformed_task(kind);
        let base = kind.split('<').next().unwrap_or(kind);
        let value_task = ["ValueTask", "System.Threading.Tasks.ValueTask"].contains(&base);
        Self::require_supported((value_task, malformed))
    }

    fn require_supported(unsupported: (bool, bool)) -> Result<(), String> {
        match unsupported {
            (false, false) => Ok(()),
            _ => Err("C# browser result requires Task of one registered client model; ValueTask and Task containers remain unsupported.".to_owned()),
        }
    }

    fn malformed_task(kind: &str) -> bool {
        Self::task_base(kind) && Self::payload(kind).is_none()
    }

    fn task_base(kind: &str) -> bool {
        ["Task", "System.Threading.Tasks.Task"].contains(&kind.split('<').next().unwrap_or(kind))
    }

    fn argument(kind: &str) -> Option<&str> {
        kind.strip_prefix(Self::TASK)
            .or_else(|| kind.strip_prefix(Self::QUALIFIED_TASK))?
            .strip_suffix('>')
            .map(str::trim)
    }

    fn registered(kind: &str, models: &ModelTable) -> Result<Option<String>, String> {
        Self::payload(kind)
            .filter(|item| *item != "BehaviorJSON")
            .map(|item| Self::record(item, models))
            .transpose()
    }

    fn scalar_fields(models: &serde_json::Value) -> Result<(), String> {
        for model in models
            .as_object()
            .into_iter()
            .flat_map(|models| models.values())
        {
            Self::fields(model)?;
        }
        Ok(())
    }

    fn fields(model: &serde_json::Value) -> Result<(), String> {
        for field in model["fields"]
            .as_object()
            .into_iter()
            .flat_map(|fields| fields.values())
        {
            Self::field(field)?;
        }
        Ok(())
    }

    fn field(field: &serde_json::Value) -> Result<(), String> {
        let scalar = field.as_str().unwrap_or("").trim_start_matches(['!', '?']);
        if !["string", "integer", "number", "boolean"].contains(&scalar) {
            return Err("C# browser Task model results currently support scalar-field records only; nested, list and arbitrary fields remain unsupported.".to_owned());
        }
        Ok(())
    }

    fn record(item: &str, models: &ModelTable) -> Result<String, String> {
        let name = item.strip_prefix("Models.").unwrap_or(item);
        if !models.is_record(name) {
            return Err(format!(
                "C# browser Task result '{item}' requires one registered client/models record; containers and unknown types are unsupported."
            ));
        }
        models.check_all()?;
        Self::scalar_fields(&models.event_models(name)?)?;
        Ok(name.to_owned())
    }
}
