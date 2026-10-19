use crate::Failure;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MacOsSlice {
    name: &'static str,
    rust_target: &'static str,
}

impl MacOsSlice {
    pub(super) const ARM64: Self = Self {
        name: "arm64",
        rust_target: "aarch64-apple-darwin",
    };
    pub(super) const X64: Self = Self {
        name: "x86_64",
        rust_target: "x86_64-apple-darwin",
    };

    const BY_BUILD_MACHINE: [Self; 2] = [Self::X64, Self::ARM64];

    pub(super) fn build_machine() -> Self {
        Self::BY_BUILD_MACHINE[usize::from(cfg!(target_arch = "aarch64"))]
    }

    pub(super) const fn name(self) -> &'static str {
        self.name
    }

    pub(super) const fn rust_target(self) -> &'static str {
        self.rust_target
    }

    pub(super) fn swift_target(self, minimum_system_version: &str) -> String {
        format!("{}-apple-macosx{minimum_system_version}", self.name)
    }

    pub(super) async fn require_rust_target(self) -> Result<(), Failure> {
        let needed_by = format!("The {} part of the macOS app", self.name);
        super::rust_standard_library::RustStandardLibrary::require(self.rust_target, &needed_by)
            .await
    }
}
