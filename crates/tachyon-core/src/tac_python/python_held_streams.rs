use super::python_provider_source::PythonProviderSource;
use crate::Failure;
use crate::events::{EventStream, HeldStreams};
use crate::route_types::ModelTable;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) struct PythonHeldStreams {
    clients: BTreeMap<String, Vec<EventStream>>,
}

impl PythonHeldStreams {
    const SERVICES: &'static str = "client/services";

    pub(super) fn read(
        providers: &[PythonProviderSource],
        models: &ModelTable,
    ) -> Result<Self, Failure> {
        let clients = providers
            .iter()
            .map(|provider| {
                provider
                    .held_streams(models)
                    .map(|streams| (provider.class.clone(), streams))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(Self { clients })
    }

    pub(super) fn describe(&self, descriptions: &mut Map<String, Value>) -> Result<(), Failure> {
        descriptions
            .iter_mut()
            .try_for_each(|(class, description)| self.add_streams(class, description))
    }

    fn add_streams(&self, class: &str, description: &mut Value) -> Result<(), Failure> {
        let streams: Vec<Value> = description["constructor"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|binding| binding["provider"].as_str())
            .flat_map(|client| self.entries(client))
            .collect();
        if streams.len() > HeldStreams::MOST_STREAMS {
            return Err(Self::too_many(class));
        }
        description["streams"] = Value::Array(streams);
        Ok(())
    }

    fn entries(&self, client: &str) -> Vec<Value> {
        self.clients
            .get(client)
            .into_iter()
            .flatten()
            .map(|stream| {
                json!({
                    "client": client,
                    "path": stream.path,
                    "event": stream.event,
                    "socket": stream.socket,
                    "models": stream.models,
                })
            })
            .collect()
    }

    fn too_many(class: &str) -> Failure {
        let path = Path::new(Self::SERVICES).join(format!("{class}.py"));
        PythonProviderSource::failure(
            &path,
            "A Python service holds at most 16 @stream_request and @socket_request methods through its clients. Split the clients it needs between two services.",
        )
    }
}
