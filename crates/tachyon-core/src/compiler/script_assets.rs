use super::portable_path::PortablePath;
use super::script_import::ScriptImport;
use super::script_imports::ScriptImports;
use super::shared_assets::SharedAssets;
use super::typescript_compiler::TypeScriptCompiler;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(super) struct ScriptAssets<'a> {
    tools_root: &'a Path,
    snapshot_root: &'a Path,
    pending: BTreeSet<PathBuf>,
    visited: BTreeSet<PathBuf>,
    files: BTreeMap<PathBuf, Vec<u8>>,
    total_bytes: usize,
}

impl<'a> ScriptAssets<'a> {
    const MAX_FILES: usize = 256;
    const MAX_FILE_BYTES: usize = 1024 * 1024;
    const MAX_TOTAL_BYTES: usize = 8 * 1024 * 1024;

    pub(super) fn new(tools_root: &'a Path, snapshot_root: &'a Path) -> Self {
        Self {
            tools_root,
            snapshot_root,
            pending: BTreeSet::new(),
            visited: BTreeSet::new(),
            files: BTreeMap::new(),
            total_bytes: 0,
        }
    }

    pub(super) fn rewrite(
        &mut self,
        source_path: &Path,
        output_path: &Path,
        bytes: Vec<u8>,
    ) -> Result<Vec<u8>, Failure> {
        let source_path = source_path
            .strip_prefix(self.snapshot_root)
            .unwrap_or(source_path);
        let mut source = String::from_utf8(bytes)
            .map_err(|_| Self::failure(source_path, "module text must be UTF-8"))?;
        ScriptImports::read(&source)
            .into_iter()
            .rev()
            .try_for_each(|import| {
                self.rewrite_import(source_path, output_path, &mut source, &import)
            })?;
        Ok(source.into_bytes())
    }

    pub(super) async fn collect(mut self) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        self.publish_pending().await?;
        Ok(self.files.into_iter().collect())
    }

    async fn publish_pending(&mut self) -> Result<(), Failure> {
        while let Some(path) = self.pending.pop_first() {
            self.publish(&path).await?;
        }
        Ok(())
    }

    fn rewrite_import(
        &mut self,
        source_path: &Path,
        output: &Path,
        source: &mut String,
        import: &ScriptImport,
    ) -> Result<(), Failure> {
        let (specifier, suffix) = Self::split_suffix(&import.specifier);
        self.target(source_path, specifier)?
            .into_iter()
            .for_each(|target| {
                let emitted = Self::output_path(&target);
                let relative = format!("{}{suffix}", Self::relative(output, &emitted));
                source.replace_range(import.range.clone(), &relative);
                self.visited
                    .insert(target.clone())
                    .then(|| self.pending.insert(target));
            });
        Ok(())
    }

    async fn publish(&mut self, path: &Path) -> Result<(), Failure> {
        let authored = self.read(path)?;
        super::shared_scripts::SharedScripts::check_file(self.snapshot_root, path, &authored)?;
        let bytes = self.compile(path, &authored).await?;
        let output = Self::output_path(path);
        self.require_new_output(path, &output)?;
        let bytes = self.rewrite(path, &output, bytes)?;
        self.files.insert(output, bytes);
        Ok(())
    }

    fn require_new_output(&self, source: &Path, output: &Path) -> Result<(), Failure> {
        if self.files.contains_key(output) {
            return Err(Self::failure(
                source,
                "two source modules claim the same output path",
            ));
        }
        Ok(())
    }

    fn target(&self, source: &Path, specifier: &str) -> Result<Option<PathBuf>, Failure> {
        Self::client_path(source, specifier)
            .filter(|path| Self::allowed_path(path))
            .map(|path| {
                Self::require_utility_language(&path)?;
                self.resolve_file(source, &path)
            })
            .transpose()
    }

    fn require_utility_language(path: &Path) -> Result<(), Failure> {
        let utility = crate::annotation_layout::UtilityLayout::is_client_utility(path);
        let module = path.extension().is_some_and(|extension| extension == "mjs");
        if [utility, module].into_iter().all(|holds| holds) {
            return Err(Self::failure(
                path,
                "utility modules use .js or .ts so their static class contract can be checked",
            ));
        }
        Ok(())
    }

    fn allowed_path(path: &Path) -> bool {
        let folder = [
            crate::annotation_layout::UtilityLayout::is_client_utility(path),
            path.starts_with("client/shared"),
        ]
        .contains(&true);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        [folder, ["js", "mjs", "ts"].contains(&extension)]
            .into_iter()
            .all(|holds| holds)
    }

    fn resolve_file(&self, source: &Path, path: &Path) -> Result<PathBuf, Failure> {
        let typed = path
            .extension()
            .is_some_and(|extension| extension == "js")
            .then(|| path.with_extension("ts"));
        [Some(path.to_path_buf()), typed]
            .into_iter()
            .flatten()
            .find(|candidate| self.snapshot_root.join(candidate).is_file())
            .ok_or_else(|| {
                Self::failure(
                    source,
                    &format!(
                        "imported module '{}' does not exist",
                        PortablePath::of(path)
                    ),
                )
            })
    }

    fn split_suffix(specifier: &str) -> (&str, &str) {
        let at = specifier.find(['?', '#']).unwrap_or(specifier.len());
        specifier.split_at(at)
    }

    fn client_path(source: &Path, specifier: &str) -> Option<PathBuf> {
        (!specifier.contains(['\\', '?', '#', '%'])).then_some(())?;
        let (mut path, tail) = specifier
            .strip_prefix('/')
            .map(|tail| (PathBuf::from("client"), tail))
            .or_else(|| Self::relative_base(source, specifier))?;
        Path::new(tail)
            .components()
            .try_for_each(|part| Self::add_part(&mut path, part))?;
        path.starts_with("client").then_some(path)
    }

    fn relative_base<'p>(source: &Path, specifier: &'p str) -> Option<(PathBuf, &'p str)> {
        specifier.starts_with('.').then_some(())?;
        Some((source.parent()?.to_path_buf(), specifier))
    }

    fn add_part(path: &mut PathBuf, part: Component<'_>) -> Option<()> {
        match part {
            Component::Normal(name) => {
                path.push(name);
                Some(())
            }
            Component::CurDir => Some(()),
            Component::ParentDir => path.pop().then_some(()),
            _ => None,
        }
    }

    fn read(&mut self, path: &Path) -> Result<Vec<u8>, Failure> {
        let absolute = self.snapshot_root.join(path);
        let metadata = fs::symlink_metadata(&absolute)
            .map_err(|error| Self::failure(path, &error.to_string()))?;
        Self::require_regular(path, &metadata)?;
        self.require_file_limits(path, metadata.len())?;
        let bytes = fs::read(&absolute).map_err(|error| Self::failure(path, &error.to_string()))?;
        self.total_bytes = self.total_bytes.saturating_add(bytes.len());
        self.require_total_limit(path)?;
        Ok(bytes)
    }

    fn require_regular(path: &Path, metadata: &fs::Metadata) -> Result<(), Failure> {
        if [metadata.is_file(), !metadata.file_type().is_symlink()].contains(&false) {
            return Err(Self::failure(
                path,
                "imports must name regular captured files",
            ));
        }
        Ok(())
    }

    fn require_file_limits(&self, path: &Path, size: u64) -> Result<(), Failure> {
        if [
            self.visited.len() <= Self::MAX_FILES,
            size <= Self::MAX_FILE_BYTES as u64,
        ]
        .contains(&false)
        {
            return Err(Self::failure(
                path,
                "module count or file size limit exceeded",
            ));
        }
        Ok(())
    }

    fn require_total_limit(&self, path: &Path) -> Result<(), Failure> {
        if self.total_bytes > Self::MAX_TOTAL_BYTES {
            return Err(Self::failure(
                path,
                "module graph exceeds its 8 MiB size limit",
            ));
        }
        Ok(())
    }

    async fn compile(&self, path: &Path, bytes: &[u8]) -> Result<Vec<u8>, Failure> {
        let source = std::str::from_utf8(bytes)
            .map_err(|_| Self::failure(path, "module text must be UTF-8"))?;
        let bytes = super::utility_module::UtilityModule::exported(path, source)?;
        if path.extension().is_none_or(|extension| extension != "ts") {
            return Ok(SharedAssets::strip_css_imports(path, bytes));
        }
        TypeScriptCompiler::transpile(
            self.tools_root,
            self.snapshot_root,
            &self.snapshot_root.join(path),
            &PortablePath::of(path),
            &bytes,
        )
        .await
    }

    fn output_path(source: &Path) -> PathBuf {
        let mut output = source
            .strip_prefix("client")
            .unwrap_or(source)
            .to_path_buf();
        if output
            .extension()
            .is_some_and(|extension| extension == "ts")
        {
            output.set_extension("js");
        }
        output
    }

    fn relative(source: &Path, target: &Path) -> String {
        let parent: Vec<_> = source
            .parent()
            .unwrap_or(Path::new(""))
            .components()
            .collect();
        let target: Vec<_> = target.components().collect();
        let shared = parent
            .iter()
            .zip(&target)
            .take_while(|(a, b)| a == b)
            .count();
        let parents = std::iter::repeat_n(String::from(".."), parent.len() - shared);
        let target = target[shared..]
            .iter()
            .map(|part| part.as_os_str().to_string_lossy().into_owned());
        let parts: Vec<_> = parents.chain(target).collect();
        let portable = parts.join("/");
        if portable.starts_with('.') {
            portable
        } else {
            format!("./{portable}")
        }
    }

    fn failure(path: &Path, detail: &str) -> Failure {
        Failure::one(diagnostic(
            1201,
            format!("Cannot publish client script dependency: {detail}."),
            Some(String::from(
                "Use existing bounded JavaScript or TypeScript modules under client/shared.",
            )),
            source_span(&PortablePath::of(path), 0, 0),
        ))
    }
}
