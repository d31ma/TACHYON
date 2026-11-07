use crate::web_runtime::{RuntimeSources, WebRuntimeStep};

pub(crate) struct LuaRuntimeSources;

impl LuaRuntimeSources {
    pub(crate) const GLUE: &'static str =
        include_str!("../../../../runtimes/lua-web/tachyon-lua.js");
    pub(crate) const NOTICES: &'static str =
        include_str!("../../../../runtimes/lua-web/THIRD_PARTY_NOTICES.txt");
    pub(crate) const RUNTIME_SOURCE: &'static str = "tachyon_lua.c";
    pub(crate) const SOURCES: RuntimeSources = RuntimeSources::new(
        WebRuntimeStep::new("Lua"),
        include_str!("../../../../runtimes/lua-web/sources.txt"),
        &[
            (
                "sources.txt",
                include_str!("../../../../runtimes/lua-web/sources.txt"),
            ),
            (
                Self::RUNTIME_SOURCE,
                include_str!("../../../../runtimes/lua-web/tachyon_lua.c"),
            ),
            ("tachyon-lua.js", Self::GLUE),
            ("THIRD_PARTY_NOTICES.txt", Self::NOTICES),
        ],
    );
}
