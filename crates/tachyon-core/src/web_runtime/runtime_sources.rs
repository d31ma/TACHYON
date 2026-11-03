use super::{SourceArchive, WebRuntimeStep};
use crate::Failure;
use sha2::{Digest, Sha256};
use std::path::Path;

type SourceFile = (&'static str, &'static str);

#[derive(Clone, Copy)]
pub(crate) struct RuntimeSources {
    pub(crate) step: WebRuntimeStep,
    pinned: &'static str,
    files: &'static [SourceFile],
}

impl RuntimeSources {
    const EMSCRIPTEN: &'static str = "emscripten";

    pub(crate) const fn new(
        step: WebRuntimeStep,
        pinned: &'static str,
        files: &'static [SourceFile],
    ) -> Self {
        Self {
            step,
            pinned,
            files,
        }
    }

    pub(crate) fn fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        self.files
            .iter()
            .for_each(|file| Self::hash_file(&mut hasher, *file));
        crate::compiler::hex_digest(hasher.finalize())
    }

    pub(crate) fn write_into(&self, folder: &Path) -> Result<(), Failure> {
        self.files
            .iter()
            .try_for_each(|(name, text)| self.step.write_file(&folder.join(name), text.as_bytes()))
    }

    pub(crate) fn archive(&self, name: &str) -> Result<SourceArchive, Failure> {
        SourceArchive::pinned(self.step, self.pinned, name)
    }

    pub(crate) fn minimum_emscripten(&self) -> String {
        self.archive(Self::EMSCRIPTEN)
            .map(|archive| archive.version)
            .unwrap_or_default()
    }

    fn hash_file(hasher: &mut Sha256, (name, text): SourceFile) {
        hasher.update(name.as_bytes());
        hasher.update(text.len().to_le_bytes());
        hasher.update(text.as_bytes());
    }
}
