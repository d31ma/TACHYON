use super::{InstallStep, PlatformNames, ToolCommand};
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostPlatform {
    names: PlatformNames,
}

impl HostPlatform {
    pub const MACOS: &'static str = "macos";
    pub const LINUX: &'static str = "linux";
    pub const WINDOWS: &'static str = "windows";
    const ARM: &'static str = "aarch64";
    const LINUX_X64: usize = 3;
    const LINUX_ARM_NAME: &'static str = "linux-arm64";
    const LINUX_X64_NAME: &'static str = "linux-x64";
    const WINDOWS_ARM_NAME: &'static str = "windows-arm64";
    const WINDOWS_X64_NAME: &'static str = "windows-x64";
    const ROWS: [PlatformNames; 6] = [
        PlatformNames {
            system: Self::MACOS,
            arm: true,
            bun: "darwin-aarch64",
            go: "darwin-arm64",
            binaryen: "arm64-macos",
            dart: "macos-arm64",
            rust: "aarch64-apple-darwin",
        },
        PlatformNames {
            system: Self::MACOS,
            arm: false,
            bun: "darwin-x64",
            go: "darwin-amd64",
            binaryen: "x86_64-macos",
            dart: "macos-x64",
            rust: "x86_64-apple-darwin",
        },
        PlatformNames {
            system: Self::LINUX,
            arm: true,
            bun: "linux-aarch64",
            go: Self::LINUX_ARM_NAME,
            binaryen: "aarch64-linux",
            dart: Self::LINUX_ARM_NAME,
            rust: "aarch64-unknown-linux-gnu",
        },
        PlatformNames {
            system: Self::LINUX,
            arm: false,
            bun: Self::LINUX_X64_NAME,
            go: "linux-amd64",
            binaryen: "x86_64-linux",
            dart: Self::LINUX_X64_NAME,
            rust: "x86_64-unknown-linux-gnu",
        },
        PlatformNames {
            system: Self::WINDOWS,
            arm: true,
            bun: "windows-aarch64",
            go: Self::WINDOWS_ARM_NAME,
            binaryen: "arm64-windows",
            dart: Self::WINDOWS_ARM_NAME,
            rust: "aarch64-pc-windows-msvc",
        },
        PlatformNames {
            system: Self::WINDOWS,
            arm: false,
            bun: Self::WINDOWS_X64_NAME,
            go: "windows-amd64",
            binaryen: "x86_64-windows",
            dart: Self::WINDOWS_X64_NAME,
            rust: "x86_64-pc-windows-msvc",
        },
    ];

    #[must_use]
    pub fn current() -> Self {
        Self::of(std::env::consts::OS, std::env::consts::ARCH == Self::ARM)
    }

    #[must_use]
    pub fn of(system: &str, arm: bool) -> Self {
        let names = Self::row_for(system, arm).unwrap_or(Self::ROWS[Self::LINUX_X64]);
        Self { names }
    }

    #[must_use]
    pub fn names(self) -> PlatformNames {
        self.names
    }

    #[must_use]
    pub fn is_windows(self) -> bool {
        self.names.system == Self::WINDOWS
    }

    #[must_use]
    pub fn is_macos(self) -> bool {
        self.names.system == Self::MACOS
    }

    #[must_use]
    pub fn is_linux(self) -> bool {
        self.names.system == Self::LINUX
    }

    #[must_use]
    pub fn executable(self, name: &str) -> String {
        format!("{name}{}", self.executable_suffix())
    }

    #[must_use]
    pub fn archive_extension(self) -> &'static str {
        self.extension_for_archives()
    }

    pub(crate) fn unpack_zip(self, archive: &Path, top: &str, into: &Path) -> Vec<InstallStep> {
        self.zip_steps(archive, top, into)
    }

    fn extension_for_archives(self) -> &'static str {
        if self.is_windows() {
            return "zip";
        }
        "tar.gz"
    }

    fn zip_steps(self, archive: &Path, top: &str, into: &Path) -> Vec<InstallStep> {
        if self.is_linux() {
            return Self::unzip_then_move(archive, top, into);
        }
        Self::tar_zip(archive, into)
    }

    fn unzip_then_move(archive: &Path, top: &str, into: &Path) -> Vec<InstallStep> {
        let folder = archive.parent().unwrap_or(archive);
        let archive_text = archive.to_string_lossy();
        let folder_text = folder.to_string_lossy();
        vec![
            InstallStep::Run(ToolCommand::new(
                "unzip",
                &["-q", "-o", &archive_text, "-d", &folder_text],
            )),
            InstallStep::MoveFolder {
                from: folder.join(top),
                to: into.to_path_buf(),
            },
        ]
    }

    fn tar_zip(archive: &Path, into: &Path) -> Vec<InstallStep> {
        let archive_text = archive.to_string_lossy();
        let into_text = into.to_string_lossy();
        vec![
            InstallStep::CreateFolder(into.to_path_buf()),
            InstallStep::Run(ToolCommand::new(
                "tar",
                &[
                    "-xf",
                    &archive_text,
                    "-C",
                    &into_text,
                    "--strip-components=1",
                ],
            )),
        ]
    }

    fn executable_suffix(self) -> &'static str {
        if self.is_windows() {
            return ".exe";
        }
        ""
    }

    fn row_for(system: &str, arm: bool) -> Option<PlatformNames> {
        Self::ROWS
            .iter()
            .copied()
            .find(|row| (row.system, row.arm) == (system, arm))
    }
}
