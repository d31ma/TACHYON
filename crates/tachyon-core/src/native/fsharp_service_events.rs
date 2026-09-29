use super::events::Prepared;
use crate::Failure;
use crate::fsharp_source::FsharpView;
use crate::project::NativeCompanion;
use std::path::Path;

pub(super) struct FsharpServiceEvents {
    pub(super) prepared: Prepared,
    publisher: bool,
    usings: String,
}

impl FsharpServiceEvents {
    const TAC: &'static str = "tac";
    const REALM: &'static str = "client";
    const PUBLISHER_TYPE: &'static str = "ApplicationEventPublisher";
    const PUBLISHER: &'static str = "public sealed class ApplicationEventPublisher {\n    public static System.Action<object, bool> Sink;\n    public void PublishEvent(object value, bool retain = false) => Sink?.Invoke(value, retain);\n    public void publishEvent(object value, bool retain = false) => PublishEvent(value, retain);\n}\n";
    const CONNECTION: &'static str = "static class FsharpEventSetup {\n    [System.Runtime.CompilerServices.ModuleInitializer]\n    internal static void Connect() { ApplicationEventPublisher.Sink = Tac.PublishModel; }\n}\n";
    const CONSTRUCTOR_REFUSAL: &'static str = "An F# Tac service takes no primary constructor parameters except its ApplicationEventPublisher (ADR 0073)";

    pub(super) fn read(view: &FsharpView, root: &Path, class: &str) -> Result<Self, Failure> {
        let publisher = Self::takes_publisher(view.text(), class)
            .map_err(|message| super::host::native_tool_failure(1405, &message))?;
        let mut module = crate::events::extract(view.path(), view.text(), Self::TAC)
            .map_err(|failure| view.failure(&failure))?;
        if let Some(module) = module.as_mut() {
            crate::events::describe(module, view.path(), root, Self::REALM)
                .map_err(|failure| view.failure(&failure))?;
        }
        let prepared = super::events::prepare(
            NativeCompanion::CSharp,
            view.text(),
            module.as_ref(),
            view.path(),
        )?;
        Ok(Self {
            prepared,
            publisher: publisher || module.is_some(),
            usings: view.authored_usings(),
        })
    }

    fn takes_publisher(shadow: &str, class: &str) -> Result<bool, String> {
        let opening = format!("public {class}(");
        let Some(start) = shadow.find(&opening) else {
            return Ok(false);
        };
        let parameters = &shadow[start + opening.len()..];
        let parameters = parameters.split(')').next().unwrap_or(parameters);
        let words: Vec<&str> = parameters.split_whitespace().collect();
        match words.as_slice() {
            [kind, _] if *kind == Self::PUBLISHER_TYPE => Ok(true),
            _ => Err(format!("{class}: {}", Self::CONSTRUCTOR_REFUSAL)),
        }
    }

    pub(super) fn runtime(&self) -> &'static str {
        self.chosen(Self::PUBLISHER)
    }

    pub(super) fn table(&self, table: &str, class: &str) -> String {
        let created = format!("new {class}()");
        let given = format!("new {class}(new {}())", Self::PUBLISHER_TYPE);
        let constructed = table.replacen(&created, &given, usize::from(self.publisher));
        format!(
            "{}{constructed}{}",
            self.usings,
            self.chosen(Self::CONNECTION)
        )
    }

    fn chosen(&self, text: &'static str) -> &'static str {
        if self.publisher { text } else { "" }
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpServiceEvents;

    #[test]
    fn only_the_publisher_may_be_a_constructor_parameter() {
        let taking = "public class SignalService { public SignalService(ApplicationEventPublisher applicationEventPublisher) { }";
        assert_eq!(
            FsharpServiceEvents::takes_publisher(taking, "SignalService"),
            Ok(true)
        );
        let plain = "public class TallyService {";
        assert_eq!(
            FsharpServiceEvents::takes_publisher(plain, "TallyService"),
            Ok(false)
        );
        let other = "public class TallyService { public TallyService(Clock clock) { }";
        assert!(
            FsharpServiceEvents::takes_publisher(other, "TallyService")
                .is_err_and(|message| message.contains("ApplicationEventPublisher"))
        );
    }
}
