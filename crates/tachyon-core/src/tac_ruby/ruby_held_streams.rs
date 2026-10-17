use crate::Failure;
use crate::events::{EventModule, EventStream, EventType};
use crate::failure::diagnostic;
use crate::ruby_source::RubyNeeds;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) struct RubyHeldStreams<'a> {
    module: &'a EventModule,
    class: &'a str,
}

impl<'a> RubyHeldStreams<'a> {
    const MOST_STREAMS: usize = 16;
    const MOST_DEPTH: usize = 8;
    const HELPERS: &'static str = "client/helpers";
    const CODE: u16 = 1605;
    const TOO_MANY: &'static str = "A Ruby service holds at most 16 @StreamRequest and @SocketRequest methods through its clients.";

    pub(crate) const fn new(module: &'a EventModule, class: &'a str) -> Self {
        Self { module, class }
    }

    pub(crate) fn refuse_helper_hooks(&self, snapshot_root: &Path) -> Result<(), Failure> {
        let helpers = Self::helper_names(&snapshot_root.join(Self::HELPERS), 0);
        self.module
            .types
            .iter()
            .filter(|event_type| helpers.contains(&event_type.name))
            .find(|helper| Self::has_hooks(helper))
            .map_or(Ok(()), |helper| Err(Self::helper_failure(&helper.name)))
    }

    pub(crate) fn describe(
        &self,
        service_source: &str,
        path: &Path,
    ) -> Result<serde_json::Value, Failure> {
        let needed: Vec<String> = RubyNeeds::typed(service_source)
            .into_iter()
            .map(|field| field.type_name)
            .collect();
        let streams: Vec<(&str, &EventStream)> = self
            .module
            .types
            .iter()
            .filter(|event_type| self.is_stream_client(event_type))
            .filter(|client| needed.contains(&client.name))
            .flat_map(|client| {
                client
                    .streams
                    .iter()
                    .filter(|stream| !stream.topic)
                    .map(|stream| (client.name.as_str(), stream))
            })
            .collect();
        Self::bounded(&streams, path)?;
        Ok(streams
            .into_iter()
            .map(|(client, stream)| Self::entry(client, stream))
            .collect())
    }

    fn bounded(streams: &[(&str, &EventStream)], path: &Path) -> Result<(), Failure> {
        if streams.len() <= Self::MOST_STREAMS {
            return Ok(());
        }
        let help = format!(
            "Split the streaming clients that {} needs between two services.",
            path.to_string_lossy()
        );
        Err(Self::failure(Self::TOO_MANY, help))
    }

    fn entry(client: &str, stream: &EventStream) -> serde_json::Value {
        serde_json::json!({
            "client": client,
            "path": stream.path,
            "event": stream.event,
            "socket": stream.socket,
            "models": stream.models,
        })
    }

    fn is_stream_client(&self, event_type: &EventType) -> bool {
        let only_streams = [
            !Self::has_hooks(event_type),
            event_type.resources.is_empty(),
            !event_type.streams.is_empty(),
        ] == [true, true, true];
        [event_type.name != self.class, only_streams] == [true, true]
    }

    fn has_hooks(event_type: &EventType) -> bool {
        [
            event_type.listeners.is_empty(),
            event_type.post_construct.is_none(),
            event_type.pre_destroy.is_none(),
        ] != [true, true, true]
    }

    fn helper_names(folder: &Path, depth: usize) -> BTreeSet<String> {
        std::fs::read_dir(folder)
            .into_iter()
            .flatten()
            .flatten()
            .flat_map(|entry| Self::names_at(&entry.path(), depth))
            .collect()
    }

    fn names_at(path: &Path, depth: usize) -> BTreeSet<String> {
        let folder = std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir());
        if [folder, depth < Self::MOST_DEPTH] == [true, true] {
            return Self::helper_names(path, depth + 1);
        }
        path.extension()
            .filter(|extension| *extension == "rb")
            .and_then(|_| path.file_stem()?.to_str())
            .map(str::to_owned)
            .into_iter()
            .collect()
    }

    fn helper_failure(helper: &str) -> Failure {
        let message = format!(
            "The Ruby Helper {helper} has an OnStart, OnStop or OnEvent method. A Ruby Tac Helper is made when its service first uses it, so the page never runs those hooks."
        );
        let help = format!(
            "Move the hook from client/helpers/{helper}.rb to the service that needs the Helper, and let it call plain Helper methods."
        );
        Self::failure(&message, help)
    }

    fn failure(message: &str, help: String) -> Failure {
        Failure::one(diagnostic(Self::CODE, message, Some(help), None))
    }
}
