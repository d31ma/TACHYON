use super::events::Prepared;
use crate::Failure;
use crate::events::EventModule;
use crate::fsharp_source::FsharpView;
use crate::project::NativeCompanion;
use std::path::Path;

pub(super) struct FsharpServiceEvents {
    pub(super) prepared: Prepared,
    constructed_with_publisher: bool,
    publisher: bool,
    usings: String,
}

impl FsharpServiceEvents {
    const TAC: &'static str = "tac";
    const REALM: &'static str = "client";
    const PUBLISHER_TYPE: &'static str = "ApplicationEventPublisher";
    const PUBLISHER: &'static str = "public sealed class ApplicationEventPublisher {\n    public static System.Action<object, bool> Sink;\n    public void PublishEvent(object value, bool retain = false) => Sink?.Invoke(value, retain);\n    public void publishEvent(object value, bool retain = false) => PublishEvent(value, retain);\n}\n";
    const CONNECTION: &'static str = "static class FsharpEventSetup {\n    [System.Runtime.CompilerServices.ModuleInitializer]\n    internal static void Connect() { ApplicationEventPublisher.Sink = Tac.PublishModel; }\n}\n";
    const CONSTRUCTOR_REFUSAL: &'static str = "An F# Tac service takes no primary constructor parameter or Needs property except its ApplicationEventPublisher (ADR 0073)";

    pub(super) fn read(view: &FsharpView, root: &Path, class: &str) -> Result<Self, Failure> {
        let publisher = Self::takes_publisher(view.text(), class)
            .map_err(|message| super::host::native_tool_failure(1405, &message))?;
        let module = Self::module(view, root)?;
        let prepared = super::events::prepare(
            NativeCompanion::CSharp,
            view.text(),
            module.as_ref(),
            view.path(),
        )?;
        Ok(Self {
            prepared,
            constructed_with_publisher: publisher,
            publisher: [publisher, module.is_some()].contains(&true),
            usings: view.authored_usings(),
        })
    }

    fn module(view: &FsharpView, root: &Path) -> Result<Option<EventModule>, Failure> {
        let mut module = crate::events::extract(view.path(), view.text(), Self::TAC)
            .map_err(|failure| view.failure(&failure))?;
        if let Some(found) = module.as_mut() {
            crate::events::describe(found, view.path(), root, Self::REALM)
                .map_err(|failure| view.failure(&failure))?;
        }
        Ok(module)
    }

    fn takes_publisher(shadow: &str, class: &str) -> Result<bool, String> {
        let opening = format!("public {class}(");
        let parameters = shadow
            .find(&opening)
            .map(|start| Self::words_until_close(&shadow[start + opening.len()..]));
        parameters.map_or(Ok(false), |words| Self::publisher_only(&words, class))
    }

    fn words_until_close(text: &str) -> Vec<&str> {
        let parameters = text.split(')').next().unwrap_or(text);
        parameters.split_whitespace().collect()
    }

    fn publisher_only(words: &[&str], class: &str) -> Result<bool, String> {
        let publisher = words.len() == 2 && words[0] == Self::PUBLISHER_TYPE;
        publisher
            .then_some(true)
            .ok_or_else(|| format!("{class}: {}", Self::CONSTRUCTOR_REFUSAL))
    }

    pub(super) fn runtime(&self) -> &'static str {
        self.chosen(Self::PUBLISHER)
    }

    pub(super) fn table(&self, table: &str, class: &str) -> String {
        let created = format!("new {class}()");
        let given = format!("new {class}(new {}())", Self::PUBLISHER_TYPE);
        let constructed = table.replacen(
            &created,
            &given,
            usize::from(self.constructed_with_publisher),
        );
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
    use crate::native::events::Prepared;

    fn events(constructed_with_publisher: bool, publisher: bool) -> FsharpServiceEvents {
        FsharpServiceEvents {
            prepared: Prepared {
                source: String::new(),
                excluded: Vec::new(),
                entries: String::new(),
            },
            constructed_with_publisher,
            publisher,
            usings: String::from("using Models;\n"),
        }
    }

    #[test]
    fn only_a_service_that_takes_the_publisher_is_given_one() {
        let table = "var Instance = new AuditService();";
        let listening = events(false, true).table(table, "AuditService");
        assert!(listening.starts_with("using Models;\nvar Instance = new AuditService();"));
        assert!(listening.contains("ApplicationEventPublisher.Sink = Tac.PublishModel;"));
        assert!(
            events(false, true)
                .runtime()
                .contains("public static System.Action<object, bool> Sink;")
        );
        let publishing = events(true, true).table(table, "AuditService");
        assert!(publishing.contains("new AuditService(new ApplicationEventPublisher());"));
        let plain = events(false, false);
        assert_eq!(
            plain.table(table, "AuditService"),
            format!("using Models;\n{table}")
        );
        assert_eq!(plain.runtime(), "");
    }

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
