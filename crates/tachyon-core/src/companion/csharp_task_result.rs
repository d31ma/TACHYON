use super::csharp_task_shape::CsharpTaskShape;
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

    fn record(item: &str, models: &ModelTable) -> Result<String, String> {
        let name = item.strip_prefix("Models.").unwrap_or(item);
        if !models.is_record(name) {
            return Err(format!(
                "C# browser Task result '{item}' requires one registered client/models record; containers and unknown types are unsupported."
            ));
        }
        models.check_all()?;
        CsharpTaskShape::validate(name, &models.event_models(name)?)?;
        Ok(name.to_owned())
    }
}
