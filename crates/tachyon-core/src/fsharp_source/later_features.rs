use super::FsharpView;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct LaterFeatures;

impl LaterFeatures {
    const MESSAGE: &'static str = "F# classes do not support {} yet (ADR 0073). Write this class in C#, or leave the annotation out.";
    const NAMES: [&'static str; 4] = [
        "local events and lifecycle hooks (OnEvent, OnStart, OnStop, EventPublisher)",
        "NeedsAll and field injection with Needs: take dependencies as primary constructor parameters",
        "behavior annotations such as Cached, Retry or AllOrNothing",
        "streams and sockets (StreamRoute, SocketRoute)",
    ];
    const STREAM_MARKERS: [&'static str; 2] = ["[StreamRoute", "[SocketRoute"];
    const TAC: &'static str = "tac";

    pub(crate) fn refuse(
        view: &FsharpView,
        source: &Path,
        contents: &str,
        environment: &str,
    ) -> Result<(), Failure> {
        let found = Self::found(view, environment).map_err(|failure| view.failure(&failure))?;
        let first = found.iter().position(|used| *used);
        first.map_or(Ok(()), |index| Err(Self::failure(source, contents, index)))
    }

    fn found(view: &FsharpView, environment: &str) -> Result<[bool; 4], Failure> {
        let path = view.path();
        let text = view.text();
        let behavior_text = Self::without_tac_roles(path, text, environment)?;
        Ok([
            crate::events::extract(path, text, environment)?.is_some(),
            crate::yon::prepare_required_args(path, text)?.is_some(),
            crate::behavior::lower(path, &behavior_text, environment)?.is_some(),
            Self::streams(text),
        ])
    }

    fn without_tac_roles(path: &Path, text: &str, environment: &str) -> Result<String, Failure> {
        if environment == Self::TAC {
            return Ok(crate::behavior::ServiceRoles::read(path, text)?.source);
        }
        Ok(text.to_owned())
    }

    fn streams(text: &str) -> bool {
        text.lines().map(str::trim_start).any(|line| {
            Self::STREAM_MARKERS
                .iter()
                .any(|marker| line.starts_with(marker))
        })
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

    fn refused(path: &str, source: &str, environment: &str) -> Option<String> {
        let path = Path::new(path);
        let view = FsharpView::new(path, source);
        LaterFeatures::refuse(&view, path, source, environment)
            .err()
            .map(|failure| failure.to_string())
    }

    #[test]
    fn streams_and_behaviors_are_refused_by_name() {
        let stream = "[<RequestRoute(\"/ticks\")>]\n[<Public>]\ntype TicksController() =\n    [<StreamRoute>]\n    member this.StreamTicks() : seq<int> = seq { yield 1 }\n";
        assert!(
            refused("server/controllers/TicksController.fs", stream, "yon")
                .is_some_and(|text| text.contains("streams and sockets"))
        );
        let cached = "[<Service>]\ntype PriceService() =\n    [<Cached>]\n    member this.Price(sku: string) : int = 1\n";
        let tac = refused("client/services/PriceService.fs", cached, "tac");
        assert!(tac.is_some(), "{tac:?}");
    }

    #[test]
    fn a_plain_service_is_accepted() {
        let plain =
            "[<Service>]\ntype PriceService() =\n    member this.Price(sku: string) : int = 1\n";
        assert!(refused("server/services/PriceService.fs", plain, "yon").is_none());
        assert!(refused("client/services/PriceService.fs", plain, "tac").is_none());
    }
}
