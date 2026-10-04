#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowsArchitecture {
    X64,
    Arm64,
}

impl Default for WindowsArchitecture {
    fn default() -> Self {
        Self::build_machine()
    }
}

impl WindowsArchitecture {
    #[must_use]
    pub const fn build_machine() -> Self {
        if cfg!(target_arch = "aarch64") {
            Self::Arm64
        } else {
            Self::X64
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::X64 => "x64",
            Self::Arm64 => "arm64",
        }
    }

    pub(super) const fn rust_target(self) -> &'static str {
        match self {
            Self::X64 => "x86_64-pc-windows-msvc",
            Self::Arm64 => "aarch64-pc-windows-msvc",
        }
    }

    pub(super) const fn webview2_library_folder(self) -> &'static str {
        match self {
            Self::X64 => "build/native/x64",
            Self::Arm64 => "build/native/arm64",
        }
    }

    pub(super) const fn dotnet_runtime(self) -> &'static str {
        match self {
            Self::X64 => "win-x64",
            Self::Arm64 => "win-arm64",
        }
    }

    pub(super) const fn linker_machine(self) -> &'static str {
        match self {
            Self::X64 => "/MACHINE:X64",
            Self::Arm64 => "/MACHINE:ARM64",
        }
    }

    pub(super) const fn artifact_architecture(self) -> &'static str {
        match self {
            Self::X64 => "x86_64",
            Self::Arm64 => "aarch64",
        }
    }

    pub(super) const fn developer_prompt(self) -> &'static str {
        match self {
            Self::X64 => "vcvarsall.bat x64 (or arm64_amd64 on an ARM64 machine)",
            Self::Arm64 => "vcvarsall.bat arm64 (or amd64_arm64 on an x64 machine)",
        }
    }
}
