use super::{
    native_ui_asset::NativeUiAsset, native_ui_bounds::NativeUiBounds,
    native_ui_descriptor::NativeUiDescriptor, native_ui_problem::NativeUiProblem,
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
    bounds: NativeUiBounds,
}

impl NativeUiCapture {
    const MAX_FILES: usize = 4096;
    const MAX_ENTRIES: usize = 8192;
    const MAX_DEPTH: usize = 32;

    pub(super) fn read(root: &Dir) -> Result<Self, Failure> {
        Self::with_bounds(root, NativeUiBounds::web())
    }

    pub(super) fn artifact(root: &Dir) -> Result<Self, Failure> {
        Self::with_bounds(root, NativeUiBounds::artifact())
    }

    pub(super) fn subtree(&self, prefix: &str) -> Result<Self, Failure> {
        self.subtree_capture(prefix)
    }

    fn subtree_capture(&self, prefix: &str) -> Result<Self, Failure> {
        let mut capture = Self::default();
        self.bytes
            .iter()
            .filter_map(|(path, bytes)| path.strip_prefix(prefix).map(|relative| (relative, bytes)))
            .try_for_each(|(relative, bytes)| {
                capture.captured(relative.to_owned(), bytes.clone())
            })?;
        capture.nonempty()?;
        Ok(capture)
    }

    fn with_bounds(root: &Dir, bounds: NativeUiBounds) -> Result<Self, Failure> {
        let mut capture = Self {
            bounds,
            ..Self::default()
        };
        capture.visit(root, Path::new(""), 0)?;
        capture
            .assets
            .sort_by(|left, right| left.path.cmp(&right.path));
        capture.nonempty()?;
        Ok(capture)
    }

    fn visit(&mut self, root: &Dir, path: &Path, depth: usize) -> Result<(), Failure> {
        Self::depth(depth)?;
        let mut entries = root
            .entries()
            .map_err(|error| NativeUiProblem::io(&error))?;
        entries.try_for_each(|entry| {
            let entry = entry.map_err(|error| NativeUiProblem::io(&error))?;
            self.entry(root, path, &entry, depth)
        })
    }

    fn entry(
        &mut self,
        root: &Dir,
        path: &Path,
        entry: &DirEntry,
        depth: usize,
    ) -> Result<(), Failure> {
        self.count_entry()?;
        let name = entry.file_name();
        let path = path.join(&name);
        let metadata = root
            .symlink_metadata(&name)
            .map_err(|error| NativeUiProblem::io(&error))?;
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
            let child = root
                .open_dir_nofollow(name)
                .map_err(|error| NativeUiProblem::io(&error))?;
            return self.visit(&child, path, depth + 1);
        }
        self.file(root, name, path)
    }

    fn file(&mut self, root: &Dir, name: &std::ffi::OsStr, path: &Path) -> Result<(), Failure> {
        let portable = Self::portable(path)?;
        let bytes = self.bounded_read(root, name)?;
        self.captured(portable, bytes)
    }

    fn captured(&mut self, portable: String, bytes: Vec<u8>) -> Result<(), Failure> {
        self.asset_size(bytes.len() as u64)?;
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

    fn bounded_read(&self, root: &Dir, name: &std::ffi::OsStr) -> Result<Vec<u8>, Failure> {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No).nonblock(true);
        let file = root
            .open_with(name, &options)
            .map_err(|error| NativeUiProblem::io(&error))?;
        let metadata = file
            .metadata()
            .map_err(|error| NativeUiProblem::io(&error))?;
        self.file_metadata(&metadata)?;
        let mut bytes = Vec::new();
        file.take(self.bounds.asset_bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| NativeUiProblem::io(&error))?;
        self.asset_size(bytes.len() as u64)?;
        Ok(bytes)
    }

    fn file_metadata(&self, metadata: &cap_std::fs::Metadata) -> Result<(), Failure> {
        NativeUiProblem::require(
            metadata.is_file(),
            "Native UI assets must be regular files.",
        )?;
        self.asset_size(metadata.len())
    }

    fn asset_size(&self, size: u64) -> Result<(), Failure> {
        if size > self.bounds.asset_bytes {
            return Err(NativeUiProblem::failure(
                "Native update asset exceeds its byte limit.",
            ));
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
        let parts = path
            .iter()
            .map(|part| {
                part.to_str()
                    .ok_or_else(|| NativeUiProblem::failure("Native UI paths must be UTF-8."))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let name = parts.join("/");
        if name.len() > 1024 || name.contains(['\\', ':', '?', '#', '%']) {
            return Err(NativeUiProblem::failure(
                "Native UI paths must be bounded canonical local asset names.",
            ));
        }
        Ok(name)
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
        if self.total_bytes > self.bounds.total_bytes {
            return Err(NativeUiProblem::failure(
                "Native update capture exceeds its total byte limit.",
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
