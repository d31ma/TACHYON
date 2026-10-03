use super::style_path::StylePath;
use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _, OpenOptionsSyncExt as _};
use cap_std::fs::{Dir, OpenOptions};
use std::io::Read as _;
use std::path::Path;

pub(super) struct StyleFile;

impl StyleFile {
    pub(super) const MAX_BYTES: usize = 1_048_576;

    pub(super) fn read(snapshot: &Path, source: &str) -> Result<Vec<u8>, String> {
        StylePath::validate(source).map_err(String::from)?;
        let root = Dir::open_ambient_dir(snapshot, cap_std::ambient_authority())
            .map_err(|error| error.to_string())?;
        let mut directory = root
            .open_dir_nofollow("client")
            .map_err(|error| error.to_string())?;
        let path = &source[1..];
        let (parents, name) = path
            .rsplit_once('/')
            .ok_or("Stylesheet requires a shared path.")?;
        Self::open_parents(parents, &mut directory)?;
        Self::read_file(&directory, name)
    }

    fn open_parents(parents: &str, directory: &mut Dir) -> Result<(), String> {
        for parent in parents.split('/') {
            *directory = directory
                .open_dir_nofollow(parent)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn read_file(directory: &Dir, name: &str) -> Result<Vec<u8>, String> {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No).nonblock(true);
        let file = directory
            .open_with(name, &options)
            .map_err(|error| error.to_string())?;
        let metadata = file.metadata().map_err(|error| error.to_string())?;
        Self::check_file(metadata.is_file(), metadata.len())?;
        let mut bytes = Vec::new();
        file.take(Self::MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        Self::check_size(bytes.len())?;
        Ok(bytes)
    }

    fn check_file(regular: bool, length: u64) -> Result<(), String> {
        if !regular || length > Self::MAX_BYTES as u64 {
            return Err(String::from(
                "Stylesheets must be regular non-symlinked files of at most 1 MiB.",
            ));
        }
        Ok(())
    }

    fn check_size(length: usize) -> Result<(), String> {
        if length > Self::MAX_BYTES {
            return Err(String::from("Stylesheet exceeds the 1 MiB source limit."));
        }
        Ok(())
    }
}
