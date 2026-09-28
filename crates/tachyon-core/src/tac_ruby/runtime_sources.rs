use super::source_archive::SourceArchive;
use crate::Failure;
use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) struct RubyRuntimeSources;

impl RubyRuntimeSources {
    pub(crate) const GLUE: &'static str =
        include_str!("../../../../runtimes/ruby-web/tachyon-ruby.js");
    pub(crate) const NOTICES: &'static str =
        include_str!("../../../../runtimes/ruby-web/THIRD_PARTY_NOTICES.txt");
    pub(crate) const BUILD_CONFIG: &'static str = "build-config.rb";
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
        let hasher = Self::hashed_files();
        crate::compiler::hex_digest(hasher.finalize())
    }

    pub(crate) fn write_into(folder: &Path) -> Result<(), Failure> {
        Self::write_files(folder)
    }

    pub(crate) fn archive(name: &str) -> Result<SourceArchive, Failure> {
        SourceArchive::pinned(Self::PINNED_SOURCES, name)
    }

    pub(crate) fn minimum_emscripten() -> String {
        SourceArchive::pinned(Self::PINNED_SOURCES, "emscripten")
            .map(|archive| archive.version)
            .unwrap_or_default()
    }

    fn hashed_files() -> Sha256 {
        let mut hasher = Sha256::new();
        for (name, text) in Self::FILES {
            hasher.update(name.as_bytes());
            hasher.update(text.len().to_le_bytes());
            hasher.update(text.as_bytes());
        }
        hasher
    }

    fn write_files(folder: &Path) -> Result<(), Failure> {
        for (name, text) in Self::FILES {
            super::runtime_step::RubyRuntimeStep::write_file(&folder.join(name), text.as_bytes())?;
        }
        Ok(())
    }
}
