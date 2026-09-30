use super::events::Prepared;
use crate::Failure;
use crate::companion::CsharpModelType;
use crate::events::EventModule;
use crate::fsharp_source::{FsharpConstructorProps, FsharpView};
use crate::project::NativeCompanion;
use std::path::Path;

pub(super) struct FsharpServiceEvents {
    pub(super) prepared: Prepared,
    arguments: String,
    publisher: bool,
    usings: String,
}

impl FsharpServiceEvents {
    const TAC: &'static str = "tac";
    const REALM: &'static str = "client";
    const PUBLISHER_TYPE: &'static str = "ApplicationEventPublisher";
    const PUBLISHER: &'static str = "public sealed class ApplicationEventPublisher {\n    public static System.Action<object, bool> Sink;\n    public void PublishEvent(object value, bool retain = false) => Sink?.Invoke(value, retain);\n    public void publishEvent(object value, bool retain = false) => PublishEvent(value, retain);\n}\n";
    const CONNECTION: &'static str = "static class FsharpEventSetup {\n    [System.Runtime.CompilerServices.ModuleInitializer]\n    internal static void Connect() { ApplicationEventPublisher.Sink = Tac.PublishModel; }\n}\n";
    const CONSTRUCTOR_REFUSAL: &'static str = "An F# Tac service takes no primary constructor parameter or Needs property except its ApplicationEventPublisher and, under [<NeedsAll>], the component's props (ADR 0073)";
    const COLLABORATOR_SUFFIX: &'static str = "Service";
    const PROP_READER: &'static str = "TacModels.Prop";

    pub(super) fn read(
        view: &FsharpView,
        root: &Path,
        class: &str,
        needs_all: bool,
    ) -> Result<Self, Failure> {
        let parameters = FsharpConstructorProps::from_shadow(view.text(), class);
        let arguments = Self::arguments(&parameters, class, needs_all)
            .map_err(|message| super::host::native_tool_failure(1405, &message))?;
        let publisher = parameters
            .iter()
            .any(|parameter| parameter.type_name == Self::PUBLISHER_TYPE);
        let module = Self::module(view, root)?;
        let prepared = super::events::prepare(
            NativeCompanion::CSharp,
            view.text(),
            module.as_ref(),
            view.path(),
        )?;
        Ok(Self {
            prepared,
            arguments,
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

    fn arguments(
        parameters: &[FsharpConstructorProps],
        class: &str,
        needs_all: bool,
    ) -> Result<String, String> {
        let mut written = Vec::new();
        for parameter in parameters {
            let argument = Self::argument(parameter, needs_all)
                .ok_or_else(|| format!("{class}: {}", Self::CONSTRUCTOR_REFUSAL))?;
            written.push(argument);
        }
        Ok(written.join(", "))
    }

    fn argument(parameter: &FsharpConstructorProps, needs_all: bool) -> Option<String> {
        let publisher = parameter.type_name == Self::PUBLISHER_TYPE;
        publisher
            .then(|| format!("new {}()", Self::PUBLISHER_TYPE))
            .or_else(|| Self::prop(parameter, needs_all))
    }

    fn prop(parameter: &FsharpConstructorProps, needs_all: bool) -> Option<String> {
        let type_name = parameter.type_name.as_str();
        let checks = [
            needs_all,
            !type_name.ends_with(Self::COLLABORATOR_SUFFIX),
            !crate::web_api_clients::WebApiClients::is_client_type(type_name),
            !CsharpModelType::is_runtime(type_name),
        ];
        let given = format!("{}(\"{}\")", Self::PROP_READER, parameter.name);
        (checks == [true; 4])
            .then(|| CsharpModelType::reading(type_name, &given))
            .flatten()
    }

    pub(super) fn runtime(&self) -> &'static str {
        self.chosen(Self::PUBLISHER)
    }

    pub(super) fn table(&self, table: &str, class: &str) -> String {
        let created = format!("new {class}()");
        let given = format!("new {class}({})", self.arguments);
        let constructed = table.replacen(&created, &given, usize::from(!self.arguments.is_empty()));
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
    use crate::fsharp_source::FsharpConstructorProps;
    use crate::native::events::Prepared;

    fn events(arguments: &str, publisher: bool) -> FsharpServiceEvents {
        FsharpServiceEvents {
            prepared: Prepared {
                source: String::new(),
                excluded: Vec::new(),
                entries: String::new(),
            },
            arguments: arguments.to_owned(),
            publisher,
            usings: String::from("using Models;\n"),
        }
    }

    fn arguments(shadow: &str, class: &str, needs_all: bool) -> Result<String, String> {
        let parameters = FsharpConstructorProps::from_shadow(shadow, class);
        FsharpServiceEvents::arguments(&parameters, class, needs_all)
    }

    #[test]
    fn only_a_service_that_takes_the_publisher_is_given_one() {
        let table = "var Instance = new AuditService();";
        let listening = events("", true).table(table, "AuditService");
        assert!(listening.starts_with("using Models;\nvar Instance = new AuditService();"));
        assert!(listening.contains("ApplicationEventPublisher.Sink = Tac.PublishModel;"));
        assert!(
            events("", true)
                .runtime()
                .contains("public static System.Action<object, bool> Sink;")
        );
        let publishing =
            events("new ApplicationEventPublisher()", true).table(table, "AuditService");
        assert!(publishing.contains("new AuditService(new ApplicationEventPublisher());"));
        let plain = events("", false);
        assert_eq!(
            plain.table(table, "AuditService"),
            format!("using Models;\n{table}")
        );
        assert_eq!(plain.runtime(), "");
    }

    #[test]
    fn only_the_publisher_may_be_a_constructor_parameter_without_needs_all() {
        let taking = "public class SignalService { public SignalService(ApplicationEventPublisher applicationEventPublisher) { }";
        assert_eq!(
            arguments(taking, "SignalService", false),
            Ok(String::from("new ApplicationEventPublisher()"))
        );
        let plain = "public class TallyService {";
        assert_eq!(arguments(plain, "TallyService", false), Ok(String::new()));
        let other = "public class TallyService { public TallyService(string label) { }";
        assert!(
            arguments(other, "TallyService", false)
                .is_err_and(|message| message.contains("NeedsAll"))
        );
    }

    #[test]
    fn needs_all_props_are_read_from_the_component_properties() {
        let shadow = "public class ShelfService { public ShelfService(FSharpList<string> files, int count) { }";
        assert_eq!(
            arguments(shadow, "ShelfService", true),
            Ok(String::from(
                "Microsoft.FSharp.Collections.ListModule.OfSeq(TacModels.ReadList<string>(TacModels.Prop(\"files\"))), Convert.ToInt32(TacModels.Prop(\"count\"))"
            ))
        );
        let collaborator =
            "public class ShelfService { public ShelfService(TallyService tally) { }";
        assert!(arguments(collaborator, "ShelfService", true).is_err());
    }

    #[test]
    fn a_behavior_runtime_constructor_parameter_is_refused_under_needs_all() {
        let runtime = "public class ShelfService { public ShelfService(BehaviorSignal signal) { }";
        assert!(
            arguments(runtime, "ShelfService", true)
                .is_err_and(|message| message.contains("NeedsAll"))
        );
    }
}
