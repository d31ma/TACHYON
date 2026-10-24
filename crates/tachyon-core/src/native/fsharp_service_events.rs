use super::csharp_stream_clients::CsharpStreamClients;
use super::events::Prepared;
use super::fsharp_service_needs::FsharpServiceNeeds;
use super::held_service::HeldService;
use crate::Failure;
use crate::behavior::HttpClientSource;
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
    const CONSTRUCTOR_REFUSAL: &'static str = "An F# Tac service takes no primary constructor parameter or Needs property except its ApplicationEventPublisher, an HTTPClient from client/remotes, another F# service of the page and, under [<NeedsAll>], the component's props (ADR 0073)";
    const MAX_CLIENTS: usize = 5;
    const TOO_MANY_CLIENTS: &'static str =
        "An F# Tac service takes at most 5 HTTPClients; split larger responsibilities (ADR 0073)";
    const COLLABORATOR_SUFFIX: &'static str = "Service";
    const PROP_READER: &'static str = "TacModels.Prop";

    pub(super) fn read(
        view: &FsharpView,
        root: &Path,
        class: &str,
        (needs_all, needs): (bool, &FsharpServiceNeeds),
    ) -> Result<Self, Failure> {
        let parameters = FsharpConstructorProps::from_shadow(view.text(), class);
        let clients = Self::clients(view, &parameters)?;
        let arguments = Self::arguments(&parameters, class, (needs_all, &clients, needs))
            .map_err(|message| super::host::native_tool_failure(1405, &message))?;
        let publisher = parameters
            .iter()
            .any(|parameter| parameter.type_name == Self::PUBLISHER_TYPE);
        let mut module = Self::module(view, root)?;
        let held = HeldService {
            class,
            language: CsharpStreamClients::FSHARP,
            path: view.path(),
        };
        let streams = module
            .as_mut()
            .map(|found| CsharpStreamClients::held_entries(found, &clients, &held))
            .transpose()
            .map_err(|failure| view.failure(&failure))?;
        let mut prepared = super::events::prepare(
            NativeCompanion::CSharp,
            view.text(),
            module.as_ref(),
            view.path(),
        )?;
        prepared.entries.push_str(streams.as_deref().unwrap_or(""));
        prepared
            .excluded
            .extend(Self::hidden(&parameters, &clients));
        prepared
            .excluded
            .extend(Self::hidden(&parameters, &Self::needed(&parameters, needs)));
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

    fn clients(
        view: &FsharpView,
        parameters: &[FsharpConstructorProps],
    ) -> Result<Vec<String>, Failure> {
        let mut clients = Vec::new();
        for parameter in parameters {
            let path = HttpClientSource::class_path(view.path(), view.text(), &parameter.type_name)
                .map_err(|failure| view.failure(&failure))?;
            clients.extend(path.map(|_| parameter.type_name.clone()));
        }
        Ok(clients)
    }

    fn needed(parameters: &[FsharpConstructorProps], needs: &FsharpServiceNeeds) -> Vec<String> {
        parameters
            .iter()
            .filter(|parameter| needs.argument(&parameter.type_name).is_some())
            .map(|parameter| parameter.type_name.clone())
            .collect()
    }

    fn hidden(parameters: &[FsharpConstructorProps], clients: &[String]) -> Vec<String> {
        parameters
            .iter()
            .filter(|parameter| clients.contains(&parameter.type_name))
            .flat_map(|parameter| [parameter.name.clone(), Self::upper_first(&parameter.name)])
            .collect()
    }

    fn upper_first(name: &str) -> String {
        let mut letters = name.chars();
        let first = letters.next().map(|letter| letter.to_ascii_uppercase());
        first.into_iter().chain(letters).collect()
    }

    fn arguments(
        parameters: &[FsharpConstructorProps],
        class: &str,
        given: (bool, &[String], &FsharpServiceNeeds),
    ) -> Result<String, String> {
        Self::bounded(class, given.1)?;
        let mut written = Vec::new();
        for parameter in parameters {
            let argument = Self::argument(parameter, given)
                .ok_or_else(|| format!("{class}: {}", Self::CONSTRUCTOR_REFUSAL))?;
            written.push(argument);
        }
        Ok(written.join(", "))
    }

    fn bounded(class: &str, clients: &[String]) -> Result<(), String> {
        (clients.len() <= Self::MAX_CLIENTS)
            .then_some(())
            .ok_or_else(|| format!("{class}: {}", Self::TOO_MANY_CLIENTS))
    }

    fn argument(
        parameter: &FsharpConstructorProps,
        given: (bool, &[String], &FsharpServiceNeeds),
    ) -> Option<String> {
        let publisher = parameter.type_name == Self::PUBLISHER_TYPE;
        let client = given.1.contains(&parameter.type_name);
        publisher
            .then(|| format!("new {}()", Self::PUBLISHER_TYPE))
            .or_else(|| client.then(|| format!("new {}()", parameter.type_name)))
            .or_else(|| given.2.argument(&parameter.type_name))
            .or_else(|| Self::prop(parameter, given.0))
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
    use super::{FsharpServiceEvents, FsharpServiceNeeds};
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
        FsharpServiceEvents::arguments(
            &parameters,
            class,
            (needs_all, &[], &FsharpServiceNeeds::default()),
        )
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
