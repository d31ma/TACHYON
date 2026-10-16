use super::fsharp_service_events::FsharpServiceEvents;
use super::fsharp_task_results::FsharpTaskResults;
use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::behavior::{DetachedCsharp, ServiceRoles};
use crate::companion::CompanionLanguage;
use crate::fsharp_source::{
    FieldNeeds, FsharpAnswer, FsharpBehaviors, FsharpMarkers, FsharpSource, FsharpView,
};
use crate::model_sources::ModelSources;
use crate::route_types::ModelTable;
use std::path::Path;

pub(super) struct FsharpService;

impl FsharpService {
    const NO_REQUIRED: &'static str = "internal static class TacServiceInfo { internal static readonly string[] Required = { }; }\ninternal static class TacServiceLifetime { internal const bool Prototype = true; }\n";
    const TAC: &'static str = "tac";

    pub(super) fn body_or(
        item: &NativeCompanionInput,
        authored: &str,
        other: impl FnOnce() -> Result<String, Failure>,
    ) -> Result<String, Failure> {
        Some(item)
            .filter(|item| FsharpSource::is_fsharp(&item.source))
            .map_or_else(other, |item| Self::body(item, authored))
    }

    fn body(item: &NativeCompanionInput, authored: &str) -> Result<String, Failure> {
        let relative = item
            .source
            .strip_prefix(&item.snapshot_root)
            .unwrap_or(&item.source);
        let assembled = crate::annotation_modules::assemble(
            &item.snapshot_root,
            relative,
            authored,
            Self::TAC,
        )?;
        let authored = assembled.as_deref().unwrap_or(authored);
        let models = ModelSources::client_metadata(&item.snapshot_root, FsharpSource::EXTENSION)?;
        let with_models = models.assembled(relative, authored)?;
        let authored = with_models.as_str();
        if !item.web_assembly {
            crate::annotation_layout::NativeWebApis::refuse(relative, authored)?;
        }
        let view = FsharpView::tac(relative, authored);
        FsharpSource::refuse_unsupported(&view, relative, authored)?;
        let tasks = FsharpTaskResults::read(
            &view,
            item.web_assembly,
            (&item.class, &Self::models(item)?),
        )?;
        let needs_all = crate::fsharp_source::FsharpConstructorProps::needs_all(authored);
        let events = FsharpServiceEvents::read(&view, &item.snapshot_root, &item.class, needs_all)?;
        let shadow_path = view.path().to_string_lossy().replace('\\', "/");
        let mut excluded = events.prepared.excluded.clone();
        excluded.extend(tasks.names);
        let table = crate::companion::owner_table(
            CompanionLanguage::CSharp.for_web(item.web_assembly),
            view.text(),
            &shadow_path,
            &excluded,
            &format!("{}{}", events.prepared.entries, tasks.entries),
            false,
        )
        .map_err(|failure| view.failure(&failure))?;
        let table = FieldNeeds::constructed(authored, &events.table(&table, &item.class));
        let detached = Self::detached(&view)?;
        let stripped = models.without_declarations(relative, &FsharpMarkers::strip(authored))?;
        let Some(detached) = detached else {
            let csharp = format!("{table}{}", Self::NO_REQUIRED);
            return Ok(FsharpSource::web_body(
                &csharp,
                &FsharpSource::with_runtime(&stripped, events.runtime()),
            ));
        };
        let fsharp = FsharpBehaviors::program(&view, &stripped, &detached)
            .map_err(|message| Self::failure(relative, &message))?;
        let csharp = format!("{table}{}{}", Self::NO_REQUIRED, detached.setups);
        let runtime = format!("{}{}", detached.runtime, events.runtime());
        Ok(FsharpSource::web_body(
            &csharp,
            &FsharpSource::with_runtime(&fsharp, &runtime),
        ))
    }

    fn models(item: &NativeCompanionInput) -> Result<ModelTable, Failure> {
        ModelTable::read_realm(&item.snapshot_root, "client", FsharpSource::EXTENSION)
            .map_err(|message| super::host::native_tool_failure(1605, &message))
    }

    fn detached(view: &FsharpView) -> Result<Option<DetachedCsharp>, Failure> {
        let roles = ServiceRoles::read(view.path(), view.text())
            .map_err(|failure| view.failure(&failure))?;
        let exchanged = FsharpAnswer::exchange_text(&roles.source);
        crate::behavior::detached_csharp(view.path(), &exchanged, Self::TAC)
            .map_err(|failure| view.failure(&failure))
    }

    fn failure(source: &Path, message: &str) -> Failure {
        super::host::native_tool_failure(
            1308,
            &format!("{}: {message}", source.to_string_lossy().replace('\\', "/")),
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::FsharpService;
    use crate::native::registry::NativeCompanionInput;
    use crate::project::NativeCompanion;

    const SERVICE: &str = "[<Service>]\ntype SignalService() =\n    [<Needs>]\n    member val Publisher : ApplicationEventPublisher = Unchecked.defaultof<ApplicationEventPublisher> with get, set\n\n    member this.Send(size: int) : unit =\n        this.Publisher.PublishEvent(size)\n";

    const UNTYPED: &str = "[<Service>]\ntype ResetService() =\n    member val Seen : int = 0 with get, set\n\n    member this.Reset() =\n        this.Seen <- 0\n";

    fn body_of(class: &str, authored: &str) -> String {
        table_for(class, authored, true).unwrap_or_else(|failure| panic!("{failure}"))
    }

    fn table_for(
        class: &str,
        authored: &str,
        web_assembly: bool,
    ) -> Result<String, crate::Failure> {
        table_with(class, authored, web_assembly, &[])
    }

    fn table_with(
        class: &str,
        authored: &str,
        web_assembly: bool,
        others: &[(&str, &str)],
    ) -> Result<String, crate::Failure> {
        let root = tempfile::tempdir().expect("project");
        for (relative, text) in others {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folders");
            std::fs::write(path, text).expect("other source");
        }
        let source = root.path().join(format!("client/services/{class}.fs"));
        std::fs::create_dir_all(source.parent().expect("parent")).expect("folders");
        std::fs::write(&source, authored).expect("source");
        let item = NativeCompanionInput {
            snapshot_root: root.path().to_path_buf(),
            language: NativeCompanion::CSharp,
            source,
            service: String::from("signal"),
            class: String::from(class),
            web_assembly,
            application_provider: false,
        };
        FsharpService::body(&item, authored)
    }

    fn unsupported_result(kind: &str) {
        let source = format!(
            "[<Service>]\ntype WaitingService() =\n    member this.Read() : {kind} =\n        Unchecked.defaultof<{kind}>\n"
        );
        let failure =
            table_for("WaitingService", &source, true).expect_err("unsupported Task shape");
        assert!(
            failure.to_string().contains("Task or ValueTask"),
            "{failure}"
        );
        assert!(failure.diagnostics().iter().any(|item| {
            item.spans
                .iter()
                .any(|span| span.file.ends_with("WaitingService.fs"))
        }));
    }

    #[test]
    fn a_method_without_a_written_type_may_return_nothing() {
        let body = body_of("ResetService", UNTYPED);
        assert!(
            body.contains(
                "TacMember.Method(arguments => TacMember.Result(() => Instance.Reset()))"
            ),
            "{body}"
        );
    }

    #[test]
    fn a_needs_publisher_property_is_set_after_the_constructor() {
        let body = body_of("SignalService", SERVICE);
        assert!(
            body.contains("new SignalService() { Publisher = new ApplicationEventPublisher() }"),
            "{body}"
        );
    }

    #[test]
    fn fsharp_browser_tasks_use_existing_continuations_while_native_keeps_sync_calls() {
        let source = "[<Service>]\ntype WaitingService() =\n    member this.Read() : Task<BehaviorJSON> =\n        Task.FromResult(BehaviorJSON.Parse(\"null\"))\n\n    member this.Pause() : Task =\n        Task.CompletedTask\n\n    member val Seen : int = 0 with get, set\n";
        let browser = body_of("WaitingService", source);
        assert!(browser.contains("TacMember.TaskMethod(async arguments =>"));
        assert!(browser.contains("return Tac.PlainJson(await Instance.Read())"));
        assert!(browser.contains("await Instance.Pause(); return null;"));
        assert!(!browser.contains("Instance.Read().GetAwaiter().GetResult()"));
        let native = table_for("WaitingService", source, false).expect("native table");
        assert!(native.contains("Instance.Read().GetAwaiter().GetResult()"));
        assert!(!native.contains("TacMember.TaskMethod("));
    }

    #[test]
    fn fsharp_browser_unknown_record_result_is_located() {
        unsupported_result("Task<Greeting>");
    }

    #[test]
    fn a_needs_http_client_is_built_by_the_page_and_hidden_from_it() {
        let client = "open System.Threading.Tasks\n\n[<HTTPClient({\"path\":\"/api/greeting\",\"setup\":\"GreetingSetup\"})>]\ntype GreetingClient =\n    [<GetRequest>]\n    abstract read: unit -> Task<BehaviorJSON>\n";
        let setup = "[<Setup>]\ntype GreetingSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        BehaviorAdapters()\n";
        let service = "open System.Threading.Tasks\n\n[<Service>]\ntype GreetingService() =\n    [<Needs>]\n    member val GreetingClient : GreetingClient = Unchecked.defaultof<GreetingClient> with get, set\n\n    member this.Ask() : Task<BehaviorJSON> =\n        this.GreetingClient.read()\n";
        let others = [
            ("client/remotes/GreetingClient.fs", client),
            ("client/setups/GreetingSetup.fs", setup),
        ];
        let body = table_with("GreetingService", service, true, &others)
            .unwrap_or_else(|failure| panic!("{failure}"));
        assert!(
            body.contains("new GreetingService() { GreetingClient = new GreetingClient() }"),
            "{body}"
        );
        assert!(!body.contains("[\"greetingClient\"]"), "{body}");
        assert!(
            body.contains("type GreetingClient(context: BehaviorContext) ="),
            "{body}"
        );
        assert!(
            body.contains("new() = GreetingClient(Behaviors.create(typeof<GreetingSetup>"),
            "{body}"
        );
    }

    #[test]
    fn fsharp_browser_map_result_is_located() {
        unsupported_result("Task<Map<string, int>>");
    }

    #[test]
    fn fsharp_browser_lists_and_value_tasks_answer_plain_values() {
        let source = "[<Service>]\ntype WaitingService() =\n    member this.Numbers() : Task<int list> =\n        Task.FromResult([ 1 ])\n\n    member this.Pause() : ValueTask =\n        ValueTask()\n";
        let browser = body_of("WaitingService", source);
        assert!(
            browser.contains("[\"numbers\"] = TacMember.TaskMethod(async arguments => { return TacModels.Plain(await Instance.Numbers()); })"),
            "{browser}"
        );
        assert!(
            browser.contains("[\"pause\"] = TacMember.TaskMethod(async arguments => { await Instance.Pause(); return null; })"),
            "{browser}"
        );
    }
}
