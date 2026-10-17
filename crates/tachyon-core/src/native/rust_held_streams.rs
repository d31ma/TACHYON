use crate::events::{EventModule, EventStream, HeldStreams};
use crate::project::NativeCompanion;

pub(super) struct RustHeldStreams;

impl RustHeldStreams {
    const TYPE_PUNCTUATION: [char; 11] = ['<', '>', ':', ',', ' ', '(', ')', '&', '[', ']', ';'];
    const TOO_MANY: &'static str = "A Rust service holds at most 16 #[StreamRequest] and #[SocketRequest] methods through its clients.";

    pub(super) fn of_owner(
        language: NativeCompanion,
        module: &EventModule,
        owner: Option<&str>,
    ) -> Result<String, &'static str> {
        let rust_owner = owner.filter(|_| language == NativeCompanion::Rust);
        rust_owner.map_or(Ok(String::new()), |owner| Self::entries(module, owner))
    }

    fn entries(module: &EventModule, owner: &str) -> Result<String, &'static str> {
        let field_types = Self::field_types(&module.source, owner);
        let streams = HeldStreams::held(&HeldStreams::clients(module, owner), &field_types);
        Self::bounded(streams).map(|streams| {
            streams
                .into_iter()
                .map(|(client, stream)| Self::entry(client, stream))
                .collect()
        })
    }

    fn bounded<'a>(
        streams: Vec<(&'a str, &'a EventStream)>,
    ) -> Result<Vec<(&'a str, &'a EventStream)>, &'static str> {
        if streams.len() > HeldStreams::MOST_STREAMS {
            return Err(Self::TOO_MANY);
        }
        Ok(streams)
    }

    fn field_types(source: &str, owner: &str) -> Vec<String> {
        crate::route_types::ModelTable::rust_model_fields(source)
            .into_iter()
            .filter(|(name, _)| name == owner)
            .flat_map(|(_, fields)| fields)
            .flat_map(|field| {
                field
                    .type_text
                    .split(Self::TYPE_PUNCTUATION)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn entry(client: &str, stream: &EventStream) -> String {
        let models = stream.models.as_ref().map_or_else(
            || String::from("None"),
            |models| format!("Some({:?})", models.to_string()),
        );
        format!(
            "(\"__stream_{client}_{}\", TacMember::Stream(TacStream {{ client: {client:?}, path: {:?}, event: {:?}, socket: {}, models: {models} }})),\n",
            stream.method, stream.path, stream.event, stream.socket
        )
    }
}
