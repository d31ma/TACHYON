use crate::wasm_optimizer::WasmOptimizer;
use crate::wasm_release_note::WasmReleaseNote;
use std::path::Path;

pub(crate) struct WasmReleaseStep {
    profile: crate::BuildProfile,
    optimizer: Option<WasmOptimizer>,
}

impl WasmReleaseStep {
    pub(crate) async fn prepare(profile: crate::BuildProfile) -> Self {
        let optimizer = Self::optimizer_for(profile).await;
        Self { profile, optimizer }
    }

    pub(crate) fn identity(&self) -> String {
        let version = self
            .optimizer
            .as_ref()
            .map_or(&[][..], WasmOptimizer::identity);
        format!("{:?} {}", self.profile, String::from_utf8_lossy(version))
    }

    pub(crate) const fn will_shrink(&self) -> bool {
        self.optimizer.is_some()
    }

    pub(crate) async fn shrink(&self, binary: &Path, flags: &[&str]) -> bool {
        Self::shrink_with(self.optimizer.as_ref(), binary, flags).await
    }

    pub(crate) fn note(&self, shrunk: bool, note: &WasmReleaseNote) -> Option<String> {
        note.for_build(self.profile, shrunk)
    }

    pub(crate) fn release_only(&self, text: String) -> Option<String> {
        Self::when_release(self.profile, text)
    }

    fn when_release(profile: crate::BuildProfile, text: String) -> Option<String> {
        (profile == crate::BuildProfile::Release).then_some(text)
    }

    async fn optimizer_for(profile: crate::BuildProfile) -> Option<WasmOptimizer> {
        match profile {
            crate::BuildProfile::Release => WasmOptimizer::find().await,
            crate::BuildProfile::Development => None,
        }
    }

    async fn shrink_with(optimizer: Option<&WasmOptimizer>, binary: &Path, flags: &[&str]) -> bool {
        match optimizer {
            Some(optimizer) => optimizer.run(binary, flags).await,
            None => false,
        }
    }
}
