#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidArchitecture {
    name: &'static str,
    abis: &'static [(&'static str, &'static str)],
    artifact_architecture: &'static str,
}

impl Default for AndroidArchitecture {
    fn default() -> Self {
        Self::UNIVERSAL
    }
}

impl AndroidArchitecture {
    pub const ARM64: Self = Self {
        name: "arm64",
        abis: &[("arm64-v8a", "aarch64-linux-android")],
        artifact_architecture: "aarch64",
    };
    pub const X64: Self = Self {
        name: "x64",
        abis: &[("x86_64", "x86_64-linux-android")],
        artifact_architecture: "x86_64",
    };
    pub const UNIVERSAL: Self = Self {
        name: "universal",
        abis: &[
            ("arm64-v8a", "aarch64-linux-android"),
            ("x86_64", "x86_64-linux-android"),
        ],
        artifact_architecture: "universal",
    };
    const BY_BUILD_MACHINE: [Self; 2] = [Self::X64, Self::ARM64];

    #[must_use]
    pub fn build_machine() -> Self {
        Self::BY_BUILD_MACHINE[usize::from(cfg!(target_arch = "aarch64"))]
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    pub(super) const fn abis(self) -> &'static [(&'static str, &'static str)] {
        self.abis
    }

    pub(super) fn gradle_abi_filters(self) -> String {
        let names: Vec<String> = self
            .abis
            .iter()
            .map(|(abi, _)| format!("\"{abi}\""))
            .collect();
        names.join(", ")
    }

    pub(super) const fn artifact_architecture(self) -> &'static str {
        self.artifact_architecture
    }
}
