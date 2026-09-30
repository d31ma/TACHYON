use super::FsharpView;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct LaterFeatures;

impl LaterFeatures {
    const MESSAGE: &'static str = "F# classes do not support {} yet (ADR 0073). Write this class in C#, or leave the annotation out.";
    const NAMES: [&'static str; 1] =
        ["NeedsAll with a list of fields: write NeedsAll alone, or mark each property with Needs"];
    const LISTED_NEEDS_ALL: &'static str = "[NeedsAll(";

    pub(crate) fn refuse(view: &FsharpView, source: &Path, contents: &str) -> Result<(), Failure> {
        let found = Self::found(view);
        let first = found.iter().position(|used| *used);
        first.map_or(Ok(()), |index| Err(Self::failure(source, contents, index)))
    }

    fn found(view: &FsharpView) -> [bool; 1] {
        [view
            .text()
            .lines()
            .any(|line| line.trim_start().starts_with(Self::LISTED_NEEDS_ALL))]
    }

    fn failure(source: &Path, contents: &str, index: usize) -> Failure {
        let portable = source.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            2015,
            Self::MESSAGE.replace("{}", Self::NAMES[index]),
            None,
            source_span(&portable, 0, contents.len()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::LaterFeatures;
    use crate::fsharp_source::FsharpView;
    use std::path::Path;

    fn refused(path: &str, source: &str) -> Option<String> {
        let path = Path::new(path);
        let view = FsharpView::new(path, source);
        LaterFeatures::refuse(&view, path, source)
            .err()
            .map(|failure| failure.to_string())
    }

    #[test]
    fn streams_sockets_and_behaviors_are_accepted() {
        let stream = "[<RequestRoute(\"/ticks\")>]\n[<Public>]\ntype TicksController() =\n    [<StreamRoute>]\n    member this.StreamTicks() : seq<int> = seq { yield 1 }\n";
        let streams = refused("server/controllers/TicksController.fs", stream);
        assert!(streams.is_none(), "{streams:?}");
        let socket = "[<SocketRoute(\"/chat\")>]\n[<Public>]\ntype ChatController() =\n    [<OnMessage>]\n    member this.OnMessage(session: SocketSession, message: ChatMessage) : unit =\n        session.Send(message)\n";
        let sockets = refused("server/controllers/ChatController.fs", socket);
        assert!(sockets.is_none(), "{sockets:?}");
        let cached = "[<Service>]\ntype PriceService() =\n    [<Cached>]\n    member this.Price(sku: string) : int = 1\n";
        let tac = refused("client/services/PriceService.fs", cached);
        assert!(tac.is_none(), "{tac:?}");
        assert!(refused("server/services/PriceService.fs", cached).is_none());
    }

    #[test]
    fn events_and_needs_all_are_accepted_and_a_needs_all_list_is_refused() {
        let events = "open Models\n\n[<Service>]\n[<NeedsAll>]\ntype AuditService(publisher: ApplicationEventPublisher) =\n    [<OnStart>]\n    member this.Start() : unit = ()\n\n    [<OnEvent>]\n    member this.Record(order: PlacedOrder) : unit = ()\n";
        let yon = refused("server/services/AuditService.fs", events);
        assert!(yon.is_none(), "{yon:?}");
        let listed = "[<Service>]\n[<NeedsAll(\"store\")>]\ntype AuditService() =\n    [<DefaultValue>]\n    val mutable store : AuditStore\n";
        let refusal = refused("server/services/AuditService.fs", listed);
        assert!(
            refusal
                .as_ref()
                .is_some_and(|text| text.contains("NeedsAll with a list of fields")),
            "{refusal:?}"
        );
    }

    #[test]
    fn a_needs_property_is_accepted() {
        let field = "[<Service>]\ntype AuditService() =\n    [<Needs>]\n    member val Store : AuditStore = Unchecked.defaultof<AuditStore> with get, set\n";
        let yon = refused("server/services/AuditService.fs", field);
        assert!(yon.is_none(), "{yon:?}");
    }

    #[test]
    fn a_plain_service_is_accepted() {
        let plain =
            "[<Service>]\ntype PriceService() =\n    member this.Price(sku: string) : int = 1\n";
        assert!(refused("server/services/PriceService.fs", plain).is_none());
        assert!(refused("client/services/PriceService.fs", plain).is_none());
    }
}
