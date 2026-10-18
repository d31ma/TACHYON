use super::{LuaConverterProviders, LuaRuntimeBuild, LuaRuntimeSources};
use crate::Failure;
use crate::events::EventModule;
use crate::failure::{diagnostic, source_span};
use crate::lua_source::{LuaBehaviorEdits, LuaSource, LuaView};
use crate::model_sources::ModelSources;
use crate::ruby_source::RubyMembers;
use crate::tac_ruby::{NeededServices, RubyWebServices};
use std::path::{Path, PathBuf};

pub(crate) struct LuaWebServices;

impl LuaWebServices {
    pub(crate) const PRELUDE: &'static str = concat!(
        include_str!("../lua_source/lua_json.lua"),
        include_str!("prelude.lua"),
        include_str!("web_apis.lua")
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
    const BEHAVIOR_RUNTIMES: [&'static str; 2] = ["", LuaBehaviorEdits::RUNTIME];
    const PUBLISHER: &'static str = "application_event_publisher";
    const TYPE_ANNOTATION: &'static str = "---@type";
    const CAPABILITY: &'static str = "Lua services do not support {} yet (ADR 0085): the page gives Lua no {} provider, so the field would stay nil. Write the service that needs it in another Tac language, such as JavaScript, TypeScript, Rust, Swift, C# or Kotlin";

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
        let (services, behaviors) = Self::services(snapshot_root, services)?;
        Ok(format!(
            "{}\n{}{}",
            Self::PRELUDE,
            Self::behavior_runtime(behaviors),
            Self::client_models(snapshot_root)?.add_to_program(&services)
        ))
    }

    pub(crate) fn behavior_runtime(used: bool) -> &'static str {
        Self::BEHAVIOR_RUNTIMES[usize::from(used)]
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
    ) -> Result<(String, bool), Failure> {
        let mut program = Self::model_names(snapshot_root)?;
        let mut behaviors = false;
        for (instance, class, source) in services {
            let (service, planned) = Self::service(snapshot_root, instance, class, source)?;
            program.push_str(&service);
            behaviors |= planned;
        }
        Ok((program, behaviors))
    }

    fn service(
        snapshot_root: &Path,
        instance: &str,
        class: &str,
        source: &Path,
    ) -> Result<(String, bool), Failure> {
        let text = std::fs::read_to_string(source)
            .map_err(|error| Self::failure(source, "", &format!("Cannot read it: {error}")))?;
        let text = crate::without_bom(&text);
        let identity = PathBuf::from(format!("client/services/{class}.lua"));
        let relative = source.strip_prefix(snapshot_root).unwrap_or(&identity);
        Self::refuse_capabilities(relative, text)?;
        let view = LuaView::joined(relative, text);
        NeededServices::LUA
            .refuse(snapshot_root, view.path(), view.text())
            .map_err(|failure| view.failure(&failure))?;
        let mut metadata = Self::metadata(snapshot_root, relative, &view)?;
        let assembled =
            crate::annotation_modules::assemble(snapshot_root, relative, text, Self::REALM)?
                .unwrap_or_else(|| text.to_owned());
        let assembled = ModelSources::client_metadata(snapshot_root, LuaSource::EXTENSION)?
            .assembled(relative, &assembled)?;
        let joined = LuaView::joined(relative, &assembled);
        let clients = crate::behavior::HttpClientSource::rewrite(joined.path(), joined.text())
            .map_err(|failure| joined.failure(&failure))?;
        let roles = crate::behavior::ServiceRoles::read(
            joined.path(),
            clients.as_deref().unwrap_or(joined.text()),
        )
        .map_err(|failure| joined.failure(&failure))?;
        let plan =
            crate::behavior::LuaBehaviorPlan::read(joined.path(), &roles.source, Self::REALM)
                .map_err(|failure| joined.failure(&failure))?;
        plan.as_ref()
            .map(|plan| LuaConverterProviders::hide_fields(&mut metadata, class, text, plan))
            .transpose()
            .map_err(|message| Self::failure(relative, text, &message))?;
        let lowered = Self::with_converters(relative, &assembled, plan.as_ref())?;
        let providers = plan
            .as_ref()
            .map(|plan| LuaConverterProviders::written(class, text, plan))
            .transpose()
            .map_err(|message| Self::failure(relative, text, &message))?
            .unwrap_or_default();
        let registered = format!(
            "\n{lowered}\n{providers}TacServices.register({}, {class}, {})\n",
            LuaSource::long_string(instance),
            LuaSource::long_string(&metadata.to_string())
        );
        Ok((registered, plan.is_some()))
    }

    fn with_converters(
        relative: &Path,
        assembled: &str,
        plan: Option<&crate::behavior::LuaBehaviorPlan>,
    ) -> Result<String, Failure> {
        plan.map(|plan| Self::lower_converters(relative, assembled, plan))
            .transpose()
            .map(|lowered| lowered.unwrap_or_else(|| assembled.to_owned()))
    }

    fn lower_converters(
        relative: &Path,
        assembled: &str,
        plan: &crate::behavior::LuaBehaviorPlan,
    ) -> Result<String, Failure> {
        LuaBehaviorEdits::tac(plan)
            .tac_program(relative, assembled)
            .map_err(|message| Self::failure(relative, assembled, &message))
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
        let code =
            crate::annotation_layout::masked_source(view.path(), &roles.source).map_err(mapped)?;
        let members = Self::public_members(RubyMembers::read(&code));
        let mut metadata = RubyWebServices::metadata(&members, events.as_ref(), &roles.required);
        metadata["writable"] = metadata["fields"].clone();
        Ok(metadata)
    }

    fn refuse_capabilities(relative: &Path, text: &str) -> Result<(), Failure> {
        let capability = text
            .lines()
            .filter_map(|line| line.trim().strip_prefix(Self::TYPE_ANNOTATION))
            .map(|declared| declared.trim().trim_end_matches('?'))
            .find(|declared| {
                crate::web_api_clients::WebApiClients::NATIVE_CAPABILITIES.contains(declared)
            });
        capability.map_or(Ok(()), |name| {
            Err(Self::failure(
                relative,
                text,
                &Self::CAPABILITY.replace("{}", name),
            ))
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
