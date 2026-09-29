pub(crate) struct FsharpEvents {
    pub(crate) glue: String,
    pub(crate) runtime: String,
}

impl FsharpEvents {
    const YON_RUNTIME: &'static str = include_str!("../events/yon/csharp.cs");
    const PUBLISHER: &'static str = "sealed class ApplicationEventPublisher";
    const PUBLIC_PUBLISHER: &'static str = "public sealed class ApplicationEventPublisher";

    pub(crate) fn detach(generated: &str) -> Self {
        let found = generated.contains(Self::YON_RUNTIME);
        let runtime =
            found.then(|| Self::YON_RUNTIME.replacen(Self::PUBLISHER, Self::PUBLIC_PUBLISHER, 1));
        Self {
            glue: generated.replacen(Self::YON_RUNTIME, "", usize::from(found)),
            runtime: runtime.unwrap_or_default(),
        }
    }

    pub(crate) fn joined_runtime(&self, behaviors: &str) -> String {
        [behaviors, self.runtime.as_str()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpEvents;

    #[test]
    fn the_event_runtime_moves_to_the_library_and_its_publisher_is_public() {
        let generated = format!(
            "\n{}\nclass __EventContainer {{}}\n",
            FsharpEvents::YON_RUNTIME
        );
        let events = FsharpEvents::detach(&generated);
        assert_eq!(events.glue, "\n\nclass __EventContainer {}\n");
        assert!(
            events
                .runtime
                .starts_with("public sealed class ApplicationEventPublisher {")
        );
        assert!(
            events
                .joined_runtime("class Behaviors {}")
                .starts_with("class Behaviors {}\npublic sealed class")
        );
    }

    #[test]
    fn glue_without_events_is_kept() {
        let events = FsharpEvents::detach("class Handler {}");
        assert_eq!(events.glue, "class Handler {}");
        assert_eq!(
            events.joined_runtime("class Behaviors {}"),
            "class Behaviors {}"
        );
        assert_eq!(events.joined_runtime(""), "");
    }
}
