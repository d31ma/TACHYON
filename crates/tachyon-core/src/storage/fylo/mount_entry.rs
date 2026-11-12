use std::path::{Path, PathBuf};

pub(super) struct MountEntry {
    point: PathBuf,
    network: bool,
}

impl MountEntry {
    #[cfg(any(target_os = "linux", test))]
    const LINUX_NETWORK_TYPES: [&'static str; 6] = ["nfs", "nfs4", "cifs", "smb3", "smbfs", "9p"];
    #[cfg(any(target_os = "linux", test))]
    const LINUX_ESCAPES: [(&'static str, &'static str); 4] = [
        ("\\040", " "),
        ("\\011", "\t"),
        ("\\012", "\n"),
        ("\\134", "\\"),
    ];
    #[cfg(any(target_os = "macos", test))]
    const MACOS_POINT_START: &'static str = " on ";
    #[cfg(any(target_os = "macos", test))]
    const MACOS_OPTIONS_START: &'static str = " (";
    #[cfg(any(target_os = "macos", test))]
    const MACOS_LOCAL_OPTION: &'static str = "local";

    #[cfg(any(target_os = "linux", test))]
    pub(super) fn from_linux_line(line: &str) -> Option<Self> {
        let mut fields = line.split(' ');
        let point = fields.nth(1)?;
        let kind = fields.next()?;
        Some(Self {
            point: PathBuf::from(Self::unescaped(point)),
            network: Self::LINUX_NETWORK_TYPES.contains(&kind),
        })
    }

    #[cfg(any(target_os = "macos", test))]
    pub(super) fn from_macos_line(line: &str) -> Option<Self> {
        let (_, mounted) = line.split_once(Self::MACOS_POINT_START)?;
        let (point, options) = mounted.rsplit_once(Self::MACOS_OPTIONS_START)?;
        Some(Self {
            point: PathBuf::from(point),
            network: !Self::local_option_in(options),
        })
    }

    pub(super) fn holds(&self, root: &Path) -> bool {
        root.starts_with(&self.point)
    }

    pub(super) fn depth(&self) -> usize {
        self.point.components().count()
    }

    pub(super) fn network(&self) -> bool {
        self.network
    }

    #[cfg(any(target_os = "linux", test))]
    fn unescaped(point: &str) -> String {
        let mut text = point.to_owned();
        for (escape, character) in Self::LINUX_ESCAPES {
            text = text.replace(escape, character);
        }
        text
    }

    #[cfg(any(target_os = "macos", test))]
    fn local_option_in(options: &str) -> bool {
        let options = options.trim_end_matches(')');
        options
            .split(", ")
            .any(|option| option == Self::MACOS_LOCAL_OPTION)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::MountEntry;

    fn linux(line: &str) -> MountEntry {
        MountEntry::from_linux_line(line).unwrap_or_else(|| panic!("linux mount line: {line}"))
    }

    fn macos(line: &str) -> MountEntry {
        MountEntry::from_macos_line(line).unwrap_or_else(|| panic!("macos mount line: {line}"))
    }

    #[test]
    fn an_efs_mount_on_linux_is_a_network_file_system() {
        let entry = linux("127.0.0.1:/ /mnt/data nfs4 rw,relatime,vers=4.1 0 0");
        assert!(entry.network());
        assert!(entry.holds(Path::new("/mnt/data/directory")));
    }

    #[test]
    fn an_smb_mount_on_linux_is_a_network_file_system() {
        assert!(linux("//server/share /mnt/share cifs rw 0 0").network());
    }

    #[test]
    fn a_local_disk_on_linux_is_not_a_network_file_system() {
        let entry = linux("/dev/vda1 / ext4 rw,relatime 0 0");
        assert!(!entry.network());
        assert_eq!(entry.depth(), 1);
    }

    #[test]
    fn a_linux_mount_point_with_a_space_is_unescaped() {
        let entry = linux("server:/ /mnt/my\\040data nfs rw 0 0");
        assert!(entry.holds(Path::new("/mnt/my data/root")));
    }

    #[test]
    fn a_mount_point_holds_only_whole_path_components() {
        let entry = linux("server:/ /mnt/data nfs rw 0 0");
        assert!(!entry.holds(Path::new("/mnt/database")));
    }

    #[test]
    fn a_macos_nfs_mount_is_a_network_file_system() {
        let entry =
            macos("10.10.10.2:/Volumes/ANVIL on /Users/me/Anvil (nfs, nodev, nosuid, nobrowse)");
        assert!(entry.network());
        assert!(entry.holds(Path::new("/Users/me/Anvil/root")));
    }

    #[test]
    fn a_macos_local_disk_is_not_a_network_file_system() {
        let entry =
            macos("/dev/disk3s5 on /System/Volumes/Data (apfs, local, journaled, nobrowse)");
        assert!(!entry.network());
        assert_eq!(entry.depth(), 4);
    }

    #[test]
    fn a_macos_mount_point_with_spaces_is_kept_whole() {
        let entry = macos("/dev/disk5s1 on /Volumes/My Disk (apfs, local, nodev, nosuid)");
        assert!(entry.holds(Path::new("/Volumes/My Disk/root")));
    }

    #[test]
    fn a_line_that_is_not_a_mount_is_ignored() {
        assert!(MountEntry::from_linux_line("broken").is_none());
        assert!(MountEntry::from_macos_line("broken").is_none());
    }
}
