use super::{EventModule, EventStream, EventType};

pub(crate) struct HeldStreams;

impl HeldStreams {
    pub(crate) const MOST_STREAMS: usize = 16;

    pub(crate) fn holds_only_streams(event_type: &EventType) -> bool {
        [
            event_type.listeners.is_empty(),
            event_type.resources.is_empty(),
            event_type.post_construct.is_none(),
            event_type.pre_destroy.is_none(),
            !event_type.streams.is_empty(),
        ]
        .iter()
        .all(|holds| *holds)
    }

    pub(crate) fn is_client(event_type: &EventType, owner: &str) -> bool {
        [
            event_type.name != owner,
            Self::holds_only_streams(event_type),
        ] == [true, true]
    }

    pub(crate) fn clients<'a>(module: &'a EventModule, owner: &str) -> Vec<&'a EventType> {
        module
            .types
            .iter()
            .filter(|event_type| Self::is_client(event_type, owner))
            .collect()
    }

    pub(crate) fn held<'a>(
        clients: &[&'a EventType],
        field_types: &[String],
    ) -> Vec<(&'a str, &'a EventStream)> {
        clients
            .iter()
            .filter(|client| field_types.contains(&client.name))
            .flat_map(|client| {
                client
                    .streams
                    .iter()
                    .filter(|stream| !stream.topic)
                    .map(|stream| (client.name.as_str(), stream))
            })
            .collect()
    }
}
