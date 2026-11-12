use std::path::Path;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::mount_entry::MountEntry;
use crate::Failure;
use crate::storage::failure;

pub(super) struct NetworkRoot;

impl NetworkRoot {
    #[cfg(target_os = "linux")]
    const MOUNT_TABLE: &'static str = "/proc/self/mounts";
    #[cfg(target_os = "macos")]
    const MOUNT_COMMAND: &'static str = "/sbin/mount";
    #[cfg(windows)]
    const SHARE_PREFIX: &'static str = r"\\";

    pub(super) fn refuse(root: &Path) -> Result<(), Failure> {
        let network = Self::is_network(root)?;
        Self::refused_when(network)
    }

    fn refused_when(network: bool) -> Result<(), Failure> {
        if network {
            return Err(failure(
                "FYLO lease \"call\" is refused on a network file system root (NFS, EFS or SMB) until FYLO qualifies it; use a local disk or lease \"server\".",
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn is_network(root: &Path) -> Result<bool, Failure> {
        let table = std::fs::read_to_string(Self::MOUNT_TABLE).map_err(|_| Self::unknown())?;
        let entries = table.lines().filter_map(MountEntry::from_linux_line);
        Ok(Self::held_by_network(entries, root))
    }

    #[cfg(target_os = "macos")]
    fn is_network(root: &Path) -> Result<bool, Failure> {
        let output = std::process::Command::new(Self::MOUNT_COMMAND)
            .output()
            .map_err(|_| Self::unknown())?;
        let table = String::from_utf8_lossy(&output.stdout);
        let entries = table.lines().filter_map(MountEntry::from_macos_line);
        Ok(Self::held_by_network(entries, root))
    }

    #[cfg(windows)]
    #[allow(clippy::unnecessary_wraps)]
    fn is_network(root: &Path) -> Result<bool, Failure> {
        Ok(root.to_string_lossy().starts_with(Self::SHARE_PREFIX))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    fn is_network(_: &Path) -> Result<bool, Failure> {
        Err(Self::unknown())
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn held_by_network(entries: impl Iterator<Item = MountEntry>, root: &Path) -> bool {
        let holder = entries
            .filter(|entry| entry.holds(root))
            .max_by_key(MountEntry::depth);
        holder.is_some_and(|entry| entry.network())
    }

    #[cfg(not(windows))]
    fn unknown() -> Failure {
        failure(
            "FYLO lease \"call\" could not confirm that the root is on a local disk; it is refused.",
        )
    }
}
