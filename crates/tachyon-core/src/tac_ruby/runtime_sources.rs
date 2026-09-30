use crate::Failure;
use crate::web_runtime::{SourceArchive, WebRuntimeStep};
use std::path::Path;

pub(crate) struct RubyRuntimeSources;

impl RubyRuntimeSources {
    pub(crate) const GLUE: &'static str =
        include_str!("../../../../runtimes/ruby-web/tachyon-ruby.js");
    pub(crate) const NOTICES: &'static str =
        include_str!("../../../../runtimes/ruby-web/THIRD_PARTY_NOTICES.txt");
    pub(crate) const BUILD_CONFIG: &'static str = "build-config.rb";
    pub(crate) const STEP: WebRuntimeStep = WebRuntimeStep::new("Ruby");
    const PINNED_SOURCES: &'static str = include_str!("../../../../runtimes/ruby-web/sources.txt");
    const FILES: [(&'static str, &'static str); 9] = [
        ("sources.txt", Self::PINNED_SOURCES),
        (
            Self::BUILD_CONFIG,
            include_str!("../../../../runtimes/ruby-web/build-config.rb"),
        ),
        (
            "build-config.rb.lock",
            include_str!("../../../../runtimes/ruby-web/build-config.rb.lock"),
        ),
        ("tachyon-ruby.js", Self::GLUE),
        ("THIRD_PARTY_NOTICES.txt", Self::NOTICES),
        (
            "tachyon-web/mrbgem.rake",
            include_str!("../../../../runtimes/ruby-web/tachyon-web/mrbgem.rake"),
        ),
        (
            "tachyon-web/mrblib/json.rb",
            include_str!("../../../../runtimes/ruby-web/tachyon-web/mrblib/json.rb"),
        ),
        (
            "tachyon-web/mrblib/prelude.rb",
            include_str!("../../../../runtimes/ruby-web/tachyon-web/mrblib/prelude.rb"),
        ),
        (
            "tachyon-web/src/tachyon_web.c",
            include_str!("../../../../runtimes/ruby-web/tachyon-web/src/tachyon_web.c"),
        ),
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
        SourceArchive::pinned(Self::STEP, Self::PINNED_SOURCES, "emscripten")
            .map(|archive| archive.version)
            .unwrap_or_default()
    }
}
