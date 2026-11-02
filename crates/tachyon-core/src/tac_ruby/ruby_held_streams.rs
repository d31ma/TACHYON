use crate::Failure;
use crate::events::{EventModule, EventStream, EventType};
use crate::failure::{diagnostic, source_span};
use crate::ruby_source::RubyNeeds;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(crate) struct RubyHeldStreams<'a> {
    module: &'a EventModule,
    class: &'a str,
    language: &'static str,
}

impl<'a> RubyHeldStreams<'a> {
    const MOST_STREAMS: usize = 16;
    const MOST_DEPTH: usize = 8;
    const HELPERS: &'static str = "client/delegates";
    const HOOKS: [&'static str; 3] = ["#[OnStart", "#[OnStop", "#[OnEvent"];
    const CODE: u16 = 1605;
    const RUBY: &'static str = "Ruby";
    const LUA: &'static str = "Lua";
    const TOO_MANY: &'static str = "A {} service holds at most 16 @StreamRequest and @SocketRequest methods through its clients.";

    pub(crate) const fn new(module: &'a EventModule, class: &'a str) -> Self {
        Self {
            module,
            class,
            language: Self::RUBY,
        }
    }

    pub(crate) const fn lua(module: &'a EventModule, class: &'a str) -> Self {
        Self {
            module,
            class,
            language: Self::LUA,
        }
    }

    pub(crate) fn refuse_helper_hooks(&self, snapshot_root: &Path) -> Result<(), Failure> {
        let helpers = Self::helper_files(&snapshot_root.join(Self::HELPERS), 0);
        self.module
            .types
            .iter()
            .filter(|event_type| Self::has_hooks(event_type))
            .find_map(|helper| helpers.get_key_value(&helper.name))
            .map_or(Ok(()), |(name, path)| {
                Err(Self::helper_failure(snapshot_root, name, path))
            })
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
        self.held(&needed, path)
    }

    pub(crate) fn held(
        &self,
        needed: &[String],
        path: &Path,
    ) -> Result<serde_json::Value, Failure> {
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
        self.bounded(&streams, path)?;
        Ok(streams
            .into_iter()
            .map(|(client, stream)| Self::entry(client, stream))
            .collect())
    }

    fn bounded(&self, streams: &[(&str, &EventStream)], path: &Path) -> Result<(), Failure> {
        if streams.len() <= Self::MOST_STREAMS {
            return Ok(());
        }
        let help = format!(
            "Split the streaming clients that {} needs between two services.",
            path.to_string_lossy()
        );
        let message = Self::TOO_MANY.replace("{}", self.language);
        Err(Self::failure(&message, help))
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

    fn helper_files(folder: &Path, depth: usize) -> BTreeMap<String, PathBuf> {
        std::fs::read_dir(folder)
            .into_iter()
            .flatten()
            .flatten()
            .flat_map(|entry| Self::files_at(&entry.path(), depth))
            .collect()
    }

    fn files_at(path: &Path, depth: usize) -> BTreeMap<String, PathBuf> {
        let folder = std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir());
        if [folder, depth < Self::MOST_DEPTH] == [true, true] {
            return Self::helper_files(path, depth + 1);
        }
        path.extension()
            .filter(|extension| *extension == "rb")
            .and_then(|_| path.file_stem()?.to_str())
            .map(|name| (name.to_owned(), path.to_path_buf()))
            .into_iter()
            .collect()
    }

    fn helper_failure(snapshot_root: &Path, helper: &str, path: &Path) -> Failure {
        let relative = path.strip_prefix(snapshot_root).unwrap_or(path);
        let portable = relative.to_string_lossy().replace('\\', "/");
        let message = format!(
            "The Ruby Delegate {helper} has an OnStart, OnStop or OnEvent method. A Ruby Tac Delegate is made when its service first uses it, so the page never runs those hooks."
        );
        let help = format!(
            "Move the hook from {portable} to the service that needs the Delegate, and let it call plain Delegate methods."
        );
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let span = Self::HOOKS
            .iter()
            .filter_map(|hook| text.find(hook).map(|at| (at, hook.len())))
            .min()
            .and_then(|(at, length)| source_span(&portable, at, at + length));
        Failure::one(diagnostic(Self::CODE, message, Some(help), span))
    }

    fn failure(message: &str, help: String) -> Failure {
        Failure::one(diagnostic(Self::CODE, message, Some(help), None))
    }
}
