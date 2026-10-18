use super::{CppFailure, CppRemote, CppService, CppStreamRequest};
use crate::Failure;
use crate::events::HeldStreams;
use crate::route_types::ModelTable;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) struct CppHeldStreams {
    clients: BTreeMap<String, Vec<Value>>,
}

impl CppHeldStreams {
    const REALM: &'static str = "client";
    const TOO_MANY: &'static str = "A C++ service holds at most 16 [[StreamRequest]] and [[SocketRequest]] methods through its clients. Split the clients it needs between two services";

    pub(crate) fn attach(root: &Path, services: &mut [CppService]) -> Result<(), Failure> {
        let written: Vec<(&CppService, Vec<CppStreamRequest>)> = services
            .iter()
            .filter(|service| service.is_remote())
            .map(|service| (service, Self::streams_of(service)))
            .filter(|(_, streams)| !streams.is_empty())
            .collect();
        let models = (!written.is_empty())
            .then(|| Self::models(root))
            .transpose()?
            .unwrap_or_default();
        let held = Self {
            clients: written
                .into_iter()
                .map(|(client, streams)| Self::described(client, &streams, &models))
                .collect::<Result<_, _>>()?,
        };
        services
            .iter_mut()
            .try_for_each(|service| held.hold(service))
    }

    fn streams_of(client: &CppService) -> Vec<CppStreamRequest> {
        CppStreamRequest::read_all(client.class(), &CppRemote::base(client.class()))
    }

    fn models(root: &Path) -> Result<ModelTable, Failure> {
        ModelTable::read_realm(root, Self::REALM, super::CppSource::EXTENSION)
            .map_err(|message| CppFailure::unreadable(Path::new(Self::REALM), 0, &message))
    }

    fn described(
        client: &CppService,
        streams: &[CppStreamRequest],
        models: &ModelTable,
    ) -> Result<(String, Vec<Value>), Failure> {
        let name = client.class().name.clone();
        let entries = streams
            .iter()
            .map(|stream| Self::entry(client, &name, stream, models))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((name, entries))
    }

    fn entry(
        client: &CppService,
        name: &str,
        stream: &CppStreamRequest,
        models: &ModelTable,
    ) -> Result<Value, Failure> {
        let described = models.event_models(&stream.event).map_err(|message| {
            let at = client.field_at(&stream.method);
            CppFailure::unreadable(
                &client.path,
                at,
                &format!("The C++ stream method '{}': {message}", stream.method),
            )
        })?;
        Ok(json!({
            "client": name,
            "path": stream.path,
            "event": stream.event,
            "socket": stream.socket,
            "models": described,
        }))
    }

    fn hold(&self, service: &mut CppService) -> Result<(), Failure> {
        let providers: BTreeSet<String> = service
            .needs()
            .into_iter()
            .map(|(_, provider)| provider)
            .collect();
        let streams: Vec<Value> = providers
            .iter()
            .filter_map(|provider| self.clients.get(provider))
            .flatten()
            .cloned()
            .collect();
        (streams.len() <= HeldStreams::MOST_STREAMS)
            .then_some(())
            .ok_or_else(|| {
                CppFailure::unreadable(&service.path, service.class().name_at, Self::TOO_MANY)
            })?;
        service.streams = streams;
        Ok(())
    }
}
