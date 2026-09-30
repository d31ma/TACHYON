use super::{LuaRuntimeBuild, LuaRuntimeSources};
use crate::Failure;
use crate::events::EventModule;
use crate::failure::{diagnostic, source_span};
use crate::lua_source::{LuaSource, LuaView};
use crate::model_sources::ModelSources;
use crate::ruby_source::RubyMembers;
use crate::tac_ruby::RubyWebServices;
use std::path::{Path, PathBuf};

pub(crate) struct LuaWebServices;

impl LuaWebServices {
    pub(crate) const PRELUDE: &'static str = concat!(
        include_str!("../lua_source/lua_json.lua"),
        include_str!("prelude.lua")
    );
    pub(crate) const RUNTIME_PATH: &'static str = "lua/tachyon-lua.wasm";
    pub(crate) const GLUE_PATH: &'static str = "lua/tachyon-lua.js";
    pub(crate) const NOTICES_PATH: &'static str = "lua/THIRD_PARTY_NOTICES.txt";
    const ENGINE: &'static str = r"const { TachyonLua } = await import('./lua/tachyon-lua.js')
const luaResponse = await fetch(new URL('./lua/tachyon-lua.wasm', import.meta.url))
const luaModule = await WebAssembly.compile(await luaResponse.arrayBuffer())
const writeLine = (line, descriptor) => { if (descriptor === 2) console.error(line); else console.log(line) }
const luaHost = (text) => {
  if (JSON.parse(text).api) return webHost(text)
  return storageHost ? storageHost(text) : JSON.stringify({ error: 'Tachyon storage is not on this page.' })
}
const lua = await TachyonLua.start(luaModule, writeLine, (payload) => { published.push(payload) }, luaHost)
const bootFailure = lua.boot(__LUA_PROGRAM__)
if (bootFailure) throw new Error(`Lua services did not start: ${JSON.parse(bootFailure).error}`)
const transport = (text) => lua.invoke(text)
";
    const REALM: &'static str = "tac";
    const CLIENT: &'static str = "client";
    const PRIVATE: char = '_';
    const PUBLISHER: &'static str = "application_event_publisher";
    const BEHAVIORS: &'static str = "Lua services do not support behavior annotations (for example Cached, Retry or AllOrNothing) yet (ADR 0085). Write this service in another Tac language, or leave the annotation out.";

    pub(crate) async fn runtime_files() -> Result<Vec<(String, Vec<u8>)>, Failure> {
        let runtime = LuaRuntimeBuild::runtime().await?;
        let glue = LuaRuntimeSources::GLUE.as_bytes().to_vec();
        let notices = LuaRuntimeSources::NOTICES.as_bytes().to_vec();
        Ok([
            (Self::RUNTIME_PATH, runtime),
            (Self::GLUE_PATH, glue),
            (Self::NOTICES_PATH, notices),
        ]
        .map(|(path, bytes)| (String::from(path), bytes))
        .into())
    }

    pub(crate) fn engine(
        snapshot_root: &Path,
        chosen: &[(String, String, PathBuf)],
    ) -> Result<String, Failure> {
        let program = Self::program(snapshot_root, chosen)?;
        let literal = serde_json::to_string(&program).unwrap_or_default();
        Ok(Self::ENGINE.replace("__LUA_PROGRAM__", &literal))
    }

    pub(crate) fn program(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
    ) -> Result<String, Failure> {
        let services = Self::services(snapshot_root, services)?;
        Ok(format!(
            "{}\n{}",
            Self::PRELUDE,
            Self::client_models(snapshot_root)?.add_to_program(&services)
        ))
    }

    pub(crate) fn client_models(snapshot_root: &Path) -> Result<ModelSources, Failure> {
        ModelSources::read(snapshot_root, Self::CLIENT, LuaSource::EXTENSION)
    }

    pub(crate) fn model_names(snapshot_root: &Path) -> Result<String, Failure> {
        crate::route_types::Converters::tac_text(LuaSource::EXTENSION, snapshot_root)
            .map_err(|message| Self::failure(Path::new("client/models"), "", &message))
    }

    fn services(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
    ) -> Result<String, Failure> {
        let mut program = Self::model_names(snapshot_root)?;
        for (instance, class, source) in services {
            program.push_str(&Self::service(snapshot_root, instance, class, source)?);
        }
        Ok(program)
    }

    fn service(
        snapshot_root: &Path,
        instance: &str,
        class: &str,
        source: &Path,
    ) -> Result<String, Failure> {
        let text = std::fs::read_to_string(source)
            .map_err(|error| Self::failure(source, "", &format!("Cannot read it: {error}")))?;
        let text = crate::without_bom(&text);
        let identity = PathBuf::from(format!("client/services/{class}.lua"));
        let relative = source.strip_prefix(snapshot_root).unwrap_or(&identity);
        let view = LuaView::joined(relative, text);
        let metadata = Self::metadata(snapshot_root, relative, &view)?;
        Ok(format!(
            "\n{text}\nTacServices.register({}, {class}, {})\n",
            LuaSource::long_string(instance),
            LuaSource::long_string(&metadata.to_string())
        ))
    }

    fn metadata(
        snapshot_root: &Path,
        relative: &Path,
        view: &LuaView,
    ) -> Result<serde_json::Value, Failure> {
        let mapped = |failure: Failure| view.failure(&failure);
        let roles =
            crate::behavior::ServiceRoles::read(view.path(), view.text()).map_err(mapped)?;
        let events = Self::events(snapshot_root, relative, view, &roles.source)?;
        let evented = events
            .as_ref()
            .map_or(roles.source.as_str(), |module| module.source.as_str());
        Self::refuse_behaviors(relative, view, evented)?;
        let code =
            crate::annotation_layout::masked_source(view.path(), &roles.source).map_err(mapped)?;
        let members = Self::public_members(RubyMembers::read(&code));
        let mut metadata = RubyWebServices::metadata(&members, events.as_ref(), &roles.required);
        metadata["writable"] = metadata["fields"].clone();
        Ok(metadata)
    }

    fn refuse_behaviors(relative: &Path, view: &LuaView, evented: &str) -> Result<(), Failure> {
        let lowered = crate::behavior::lower(view.path(), evented, Self::REALM)
            .map_err(|failure| view.failure(&failure))?;
        lowered.map_or(Ok(()), |_| {
            Err(Self::failure(relative, view.text(), Self::BEHAVIORS))
        })
    }

    fn events(
        snapshot_root: &Path,
        relative: &Path,
        view: &LuaView,
        without_roles: &str,
    ) -> Result<Option<EventModule>, Failure> {
        let mut events = crate::events::extract(view.path(), without_roles, Self::REALM)
            .map_err(|failure| view.failure(&failure))?;
        if let Some(module) = events.as_mut() {
            crate::events::describe(module, relative, snapshot_root, Self::CLIENT)?;
        }
        Ok(events)
    }

    fn public_members(mut members: RubyMembers) -> RubyMembers {
        members.methods.retain(|name| Self::is_public(name));
        members.fields.retain(|name| Self::is_page_field(name));
        members
    }

    fn is_public(name: &str) -> bool {
        !name.starts_with(Self::PRIVATE)
    }

    fn is_page_field(name: &str) -> bool {
        [Self::is_public(name), name != Self::PUBLISHER] == [true, true]
    }

    fn failure(path: &Path, text: &str, message: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            2015,
            format!("Cannot build the Lua service '{portable}': {message}"),
            None,
            source_span(&portable, 0, text.len()),
        ))
    }
}
