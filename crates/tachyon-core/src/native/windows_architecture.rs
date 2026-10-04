#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowsArchitecture {
    name: &'static str,
    rust_target: &'static str,
    webview2_library_folder: &'static str,
    dotnet_runtime: &'static str,
    linker_machine: &'static str,
    artifact_architecture: &'static str,
    developer_prompt: &'static str,
}

impl Default for WindowsArchitecture {
    fn default() -> Self {
        Self::build_machine()
    }
}

impl WindowsArchitecture {
    pub const X64: Self = Self {
        name: "x64",
        rust_target: "x86_64-pc-windows-msvc",
        webview2_library_folder: "build/native/x64",
        dotnet_runtime: "win-x64",
        linker_machine: "/MACHINE:X64",
        artifact_architecture: "x86_64",
        developer_prompt: "vcvarsall.bat x64 (or arm64_amd64 on an ARM64 machine)",
    };
    pub const ARM64: Self = Self {
        name: "arm64",
        rust_target: "aarch64-pc-windows-msvc",
        webview2_library_folder: "build/native/arm64",
        dotnet_runtime: "win-arm64",
        linker_machine: "/MACHINE:ARM64",
        artifact_architecture: "aarch64",
        developer_prompt: "vcvarsall.bat arm64 (or amd64_arm64 on an x64 machine)",
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

    pub(super) const fn rust_target(self) -> &'static str {
        self.rust_target
    }

    pub(super) const fn webview2_library_folder(self) -> &'static str {
        self.webview2_library_folder
    }

    pub(super) const fn dotnet_runtime(self) -> &'static str {
        self.dotnet_runtime
    }

    pub(super) const fn linker_machine(self) -> &'static str {
        self.linker_machine
    }

    pub(super) const fn artifact_architecture(self) -> &'static str {
        self.artifact_architecture
    }

    pub(super) const fn developer_prompt(self) -> &'static str {
        self.developer_prompt
    }
}
