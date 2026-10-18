use super::mac_os_slice::MacOsSlice;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MacOsArchitecture {
    name: &'static str,
    slices: &'static [MacOsSlice],
    artifact_architecture: &'static str,
}

impl Default for MacOsArchitecture {
    fn default() -> Self {
        Self::build_machine()
    }
}

impl MacOsArchitecture {
    pub const X64: Self = Self {
        name: "x64",
        slices: &[MacOsSlice::X64],
        artifact_architecture: "x86_64",
    };
    pub const ARM64: Self = Self {
        name: "arm64",
        slices: &[MacOsSlice::ARM64],
        artifact_architecture: "aarch64",
    };
    pub const UNIVERSAL: Self = Self {
        name: "universal",
        slices: &[MacOsSlice::ARM64, MacOsSlice::X64],
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

    pub(super) const fn slices(self) -> &'static [MacOsSlice] {
        self.slices
    }

    pub(super) fn is_build_machine(self) -> bool {
        self == Self::build_machine()
    }

    pub(super) const fn artifact_architecture(self) -> &'static str {
        self.artifact_architecture
    }
}
