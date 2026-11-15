use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::system_disk::SystemDisk;

pub(super) struct FyloRoot {
    path: PathBuf,
    system_disk: SystemDisk,
}

impl FyloRoot {
    const ROOT_OPTION: &'static str = "--root";

    pub(super) fn new(path: PathBuf, system_disk: SystemDisk) -> Self {
        Self { path, system_disk }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn arguments(&self) -> Vec<OsString> {
        let mut arguments = vec![
            OsString::from(Self::ROOT_OPTION),
            self.path.as_os_str().to_owned(),
        ];
        arguments.extend(self.system_disk.options().iter().map(OsString::from));
        arguments
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{FyloRoot, SystemDisk};

    fn text(root: &FyloRoot) -> Vec<String> {
        root.arguments()
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_root_on_a_data_disk_names_only_the_root() {
        let root = FyloRoot::new(PathBuf::from("/data/root"), SystemDisk::Refused);
        assert_eq!(text(&root), ["--root", "/data/root"]);
    }

    #[test]
    fn a_root_allowed_on_the_system_disk_adds_the_fylo_opt_out() {
        let root = FyloRoot::new(PathBuf::from("/data/root"), SystemDisk::Allowed);
        assert_eq!(text(&root), ["--root", "/data/root", "--allow-system-disk"]);
    }
}
