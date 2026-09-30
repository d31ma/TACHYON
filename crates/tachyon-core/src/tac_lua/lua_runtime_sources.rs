use crate::Failure;
use crate::web_runtime::{SourceArchive, WebRuntimeStep};
use std::path::Path;

pub(crate) struct LuaRuntimeSources;

impl LuaRuntimeSources {
    pub(crate) const GLUE: &'static str =
        include_str!("../../../../runtimes/lua-web/tachyon-lua.js");
    pub(crate) const NOTICES: &'static str =
        include_str!("../../../../runtimes/lua-web/THIRD_PARTY_NOTICES.txt");
    pub(crate) const RUNTIME_SOURCE: &'static str = "tachyon_lua.c";
    pub(crate) const STEP: WebRuntimeStep = WebRuntimeStep::new("Lua");
    const PINNED_SOURCES: &'static str = include_str!("../../../../runtimes/lua-web/sources.txt");
    const FILES: [(&'static str, &'static str); 4] = [
        ("sources.txt", Self::PINNED_SOURCES),
        (
            Self::RUNTIME_SOURCE,
            include_str!("../../../../runtimes/lua-web/tachyon_lua.c"),
        ),
        ("tachyon-lua.js", Self::GLUE),
        ("THIRD_PARTY_NOTICES.txt", Self::NOTICES),
    ];

    pub(crate) fn fingerprint() -> String {
        WebRuntimeStep::fingerprint(&Self::FILES)
    }

    pub(crate) fn write_into(folder: &Path) -> Result<(), Failure> {
        Self::STEP.write_files(folder, &Self::FILES)
    }

    pub(crate) fn archive(name: &str) -> Result<SourceArchive, Failure> {
        SourceArchive::pinned(Self::STEP, Self::PINNED_SOURCES, name)
    }

    pub(crate) fn minimum_emscripten() -> String {
        Self::archive("emscripten")
            .map(|archive| archive.version)
            .unwrap_or_default()
    }
}
