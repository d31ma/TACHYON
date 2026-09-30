use super::fsharp_service_events::FsharpServiceEvents;
use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::behavior::{DetachedCsharp, ServiceRoles};
use crate::companion::CompanionLanguage;
use crate::fsharp_source::{FieldNeeds, FsharpBehaviors, FsharpMarkers, FsharpSource, FsharpView};
use std::path::Path;

pub(super) struct FsharpService;

impl FsharpService {
    const NO_REQUIRED: &'static str = "internal static class TacServiceInfo { internal static readonly string[] Required = { }; }\n";
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
        let view = FsharpView::tac(relative, authored);
        FsharpSource::refuse_unsupported(&view, relative, authored)?;
        let events = FsharpServiceEvents::read(&view, &item.snapshot_root, &item.class)?;
        let shadow_path = view.path().to_string_lossy().replace('\\', "/");
        let table = crate::companion::owner_table(
            CompanionLanguage::CSharp,
            view.text(),
            &shadow_path,
            &events.prepared.excluded,
            &events.prepared.entries,
            false,
        )
        .map_err(|failure| view.failure(&failure))?;
        let table = FieldNeeds::constructed(authored, &events.table(&table, &item.class));
        let detached = Self::detached(&view)?;
        let stripped = FsharpMarkers::strip(authored);
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

    fn detached(view: &FsharpView) -> Result<Option<DetachedCsharp>, Failure> {
        let roles = ServiceRoles::read(view.path(), view.text())
            .map_err(|failure| view.failure(&failure))?;
        crate::behavior::detached_csharp(view.path(), &roles.source, Self::TAC)
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

    #[test]
    fn a_needs_publisher_property_is_set_after_the_constructor() {
        let root = tempfile::tempdir().expect("project");
        let source = root.path().join("client/services/SignalService.fs");
        std::fs::create_dir_all(source.parent().expect("parent")).expect("folders");
        std::fs::write(&source, SERVICE).expect("source");
        let item = NativeCompanionInput {
            snapshot_root: root.path().to_path_buf(),
            language: NativeCompanion::CSharp,
            source,
            service: String::from("signal"),
            class: String::from("SignalService"),
            web_assembly: true,
        };
        let body =
            FsharpService::body(&item, SERVICE).unwrap_or_else(|failure| panic!("{failure}"));
        assert!(
            body.contains("new SignalService() { Publisher = new ApplicationEventPublisher() }"),
            "{body}"
        );
    }
}
