use super::{
    native_ui_asset::NativeUiAsset, native_ui_descriptor::NativeUiDescriptor,
    native_ui_problem::NativeUiProblem,
};
use crate::Failure;
use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _, OpenOptionsSyncExt as _};
use cap_std::fs::{Dir, DirEntry, OpenOptions};
use std::{collections::BTreeMap, io::Read as _, path::Path};

#[derive(Default)]
pub(super) struct NativeUiCapture {
    pub(super) assets: Vec<NativeUiAsset>,
    pub(super) bytes: BTreeMap<String, Vec<u8>>,
    total_bytes: u64,
    entries: usize,
}

impl NativeUiCapture {
    const MAX_ASSET_BYTES: u64 = 16 * 1024 * 1024;
    const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
    const MAX_FILES: usize = 4096;
    const MAX_ENTRIES: usize = 8192;
    const MAX_DEPTH: usize = 32;

    pub(super) fn read(root: &Dir) -> Result<Self, Failure> {
        let mut capture = Self::default();
        capture.visit(root, Path::new(""), 0)?;
        capture
            .assets
            .sort_by(|left, right| left.path.cmp(&right.path));
        capture.nonempty()?;
        Ok(capture)
    }

    fn visit(&mut self, root: &Dir, path: &Path, depth: usize) -> Result<(), Failure> {
        Self::depth(depth)?;
        let mut entries = root.entries().map_err(NativeUiProblem::io)?;
        entries.try_for_each(|entry| {
            self.entry(root, path, entry.map_err(NativeUiProblem::io)?, depth)
        })
    }

    fn entry(
        &mut self,
        root: &Dir,
        path: &Path,
        entry: DirEntry,
        depth: usize,
    ) -> Result<(), Failure> {
        self.count_entry()?;
        let name = entry.file_name();
        let path = path.join(&name);
        let metadata = root.symlink_metadata(&name).map_err(NativeUiProblem::io)?;
        Self::regular(&metadata)?;
        self.found(root, &path, &metadata, depth)
    }

    fn found(
        &mut self,
        root: &Dir,
        path: &Path,
        metadata: &cap_std::fs::Metadata,
        depth: usize,
    ) -> Result<(), Failure> {
        let name = path.file_name().unwrap_or_default();
        if metadata.is_dir() {
            let child = root.open_dir_nofollow(name).map_err(NativeUiProblem::io)?;
            return self.visit(&child, path, depth + 1);
        }
        self.file(root, name, path)
    }

    fn file(&mut self, root: &Dir, name: &std::ffi::OsStr, path: &Path) -> Result<(), Failure> {
        let portable = Self::portable(path)?;
        let bytes = Self::bounded_read(root, name)?;
        self.budget(bytes.len() as u64)?;
        let asset = NativeUiAsset {
            path: portable.clone(),
            size: bytes.len() as u64,
            sha256: NativeUiDescriptor::sha256(&bytes),
        };
        self.assets.push(asset);
        self.bytes.insert(portable, bytes);
        Ok(())
    }

    fn bounded_read(root: &Dir, name: &std::ffi::OsStr) -> Result<Vec<u8>, Failure> {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No).nonblock(true);
        let file = root
            .open_with(name, &options)
            .map_err(NativeUiProblem::io)?;
        let metadata = file.metadata().map_err(NativeUiProblem::io)?;
        Self::file_metadata(&metadata)?;
        let mut bytes = Vec::new();
        file.take(Self::MAX_ASSET_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(NativeUiProblem::io)?;
        Self::asset_size(bytes.len() as u64)?;
        Ok(bytes)
    }

    fn file_metadata(metadata: &cap_std::fs::Metadata) -> Result<(), Failure> {
        NativeUiProblem::require(
            metadata.is_file(),
            "Native UI assets must be regular files.",
        )?;
        Self::asset_size(metadata.len())
    }

    fn asset_size(size: u64) -> Result<(), Failure> {
        if size > Self::MAX_ASSET_BYTES {
            return Err(NativeUiProblem::failure("Native UI asset exceeds 16 MiB."));
        }
        Ok(())
    }

    fn regular(metadata: &cap_std::fs::Metadata) -> Result<(), Failure> {
        NativeUiProblem::require(
            !metadata.is_symlink(),
            "Native UI generation rejects symlinks and special files.",
        )?;
        NativeUiProblem::require(
            [metadata.is_dir(), metadata.is_file()].contains(&true),
            "Native UI generation rejects symlinks and special files.",
        )
    }

    fn portable(path: &Path) -> Result<String, Failure> {
        let name = path
            .to_str()
            .ok_or_else(|| NativeUiProblem::failure("Native UI paths must be UTF-8."))?;
        if name.len() > 1024 || name.contains(['\\', ':', '?', '#', '%']) {
            return Err(NativeUiProblem::failure(
                "Native UI paths must be bounded canonical local asset names.",
            ));
        }
        Ok(name.to_owned())
    }

    fn count_entry(&mut self) -> Result<(), Failure> {
        self.entries += 1;
        if self.entries > Self::MAX_ENTRIES {
            return Err(NativeUiProblem::failure(
                "Native UI generation exceeds 8192 directory entries.",
            ));
        }
        Ok(())
    }

    fn budget(&mut self, new_bytes: u64) -> Result<(), Failure> {
        self.file_count()?;
        self.total(new_bytes)
    }

    fn total(&mut self, new_bytes: u64) -> Result<(), Failure> {
        self.total_bytes += new_bytes;
        if self.total_bytes > Self::MAX_TOTAL_BYTES {
            return Err(NativeUiProblem::failure(
                "Native UI generation exceeds 64 MiB.",
            ));
        }
        Ok(())
    }

    fn file_count(&self) -> Result<(), Failure> {
        if self.assets.len() >= Self::MAX_FILES {
            return Err(NativeUiProblem::failure(
                "Native UI generation exceeds 4096 files.",
            ));
        }
        Ok(())
    }

    fn nonempty(&self) -> Result<(), Failure> {
        if self.assets.is_empty() {
            return Err(NativeUiProblem::failure(
                "Native UI generation has no assets.",
            ));
        }
        Ok(())
    }

    fn depth(depth: usize) -> Result<(), Failure> {
        if depth > Self::MAX_DEPTH {
            return Err(NativeUiProblem::failure(
                "Native UI generation exceeds depth 32.",
            ));
        }
        Ok(())
    }
}
