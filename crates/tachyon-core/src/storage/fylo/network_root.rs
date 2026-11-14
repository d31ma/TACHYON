#[cfg(windows)]
use std::path::Component;
use std::path::Path;
#[cfg(any(windows, test))]
use std::path::Prefix;

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
    fn is_network(root: &Path) -> Result<bool, Failure> {
        let real = std::fs::canonicalize(root).map_err(|_| Self::unknown())?;
        let prefix = real.components().next().and_then(Self::prefix_of);
        Self::network_prefix(prefix)
    }

    #[cfg(windows)]
    fn prefix_of(component: Component<'_>) -> Option<Prefix<'_>> {
        let Component::Prefix(prefix) = component else {
            return None;
        };
        Some(prefix.kind())
    }

    #[cfg(any(windows, test))]
    fn network_prefix(prefix: Option<Prefix<'_>>) -> Result<bool, Failure> {
        match prefix {
            Some(Prefix::Disk(_) | Prefix::VerbatimDisk(_)) => Ok(false),
            Some(Prefix::UNC(..) | Prefix::VerbatimUNC(..)) => Ok(true),
            _ => Err(Self::unknown()),
        }
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

    fn unknown() -> Failure {
        failure(
            "FYLO lease \"call\" could not confirm that the root is on a local disk; it is refused.",
        )
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Prefix;

    use super::NetworkRoot;

    #[test]
    fn a_windows_drive_letter_is_a_local_disk() {
        let local = NetworkRoot::network_prefix(Some(Prefix::VerbatimDisk(b'C')));
        assert_eq!(local.ok(), Some(false));
    }

    #[test]
    fn a_windows_network_share_is_a_network_file_system() {
        let share = Prefix::VerbatimUNC(OsStr::new("server"), OsStr::new("data"));
        assert_eq!(NetworkRoot::network_prefix(Some(share)).ok(), Some(true));
    }

    #[test]
    fn a_windows_device_path_is_refused_as_unknown() {
        let device = Prefix::DeviceNS(OsStr::new("PhysicalDrive0"));
        assert!(NetworkRoot::network_prefix(Some(device)).is_err());
        assert!(NetworkRoot::network_prefix(None).is_err());
    }
}
