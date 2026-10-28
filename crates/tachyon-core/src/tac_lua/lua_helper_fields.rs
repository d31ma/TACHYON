use super::LuaConverterProviders;
use crate::Failure;
use crate::behavior::LuaBehaviorPlan;
use crate::failure::{diagnostic, source_span};
use crate::lua_source::LuaFields;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) struct LuaHelperFields {
    helpers: Vec<(String, PathBuf, String)>,
}

impl LuaHelperFields {
    const FOLDER: &'static str = "client/delegates";
    const EXTENSION: &'static str = "lua";
    const MOST_DEPTH: usize = 8;
    const HOOKS: [&'static str; 3] = ["-- @OnStart", "-- @OnStop", "-- @OnEvent"];
    const CODE: u16 = 1605;

    pub(super) fn read(snapshot_root: &Path) -> Result<Self, Failure> {
        let helpers = Self::files(snapshot_root, Path::new(Self::FOLDER), 0);
        let found = Self { helpers };
        found.refuse_hooks()?;
        Ok(found)
    }

    pub(super) fn written(
        &self,
        class: &str,
        source: &str,
        plan: Option<&LuaBehaviorPlan>,
    ) -> Result<String, String> {
        let providers = plan
            .map(|plan| self.providers(plan))
            .transpose()?
            .unwrap_or_default();
        let helpers = self
            .helpers
            .iter()
            .map(|(name, _, text)| (name.as_str(), text.as_str()));
        let tables: String = std::iter::once((class, source))
            .chain(helpers)
            .map(|(owner, text)| self.table(owner, text))
            .collect();
        Ok(format!("{providers}{tables}"))
    }

    pub(super) fn hide_fields(&self, metadata: &mut Value, class: &str, source: &str) {
        LuaConverterProviders::hide_named_fields(metadata, &self.fields(class, source));
    }

    fn providers(&self, plan: &LuaBehaviorPlan) -> Result<String, String> {
        self.helpers
            .iter()
            .map(|(name, _, text)| LuaConverterProviders::written(name, text, plan))
            .collect()
    }

    fn table(&self, owner: &str, source: &str) -> String {
        let fields = self.fields(owner, source);
        if fields.is_empty() {
            return String::new();
        }
        let entries = LuaConverterProviders::entries(&fields);
        format!("{owner}.__tachyonHelpers = {{ {entries} }}\n")
    }

    fn fields(&self, owner: &str, source: &str) -> Vec<(String, String)> {
        LuaFields::without_values(source, owner)
            .into_iter()
            .filter(|(_, _, marked)| *marked)
            .filter(|(_, name, _)| self.is_helper(name))
            .map(|(field, name, _)| (field, name))
            .collect()
    }

    fn is_helper(&self, name: &str) -> bool {
        self.helpers.iter().any(|(helper, _, _)| helper == name)
    }

    fn refuse_hooks(&self) -> Result<(), Failure> {
        self.helpers
            .iter()
            .find_map(|(name, path, text)| {
                Self::hook(text).map(|(at, hook)| Self::hook_failure(name, path, at, hook))
            })
            .map_or(Ok(()), Err)
    }

    fn hook(text: &str) -> Option<(usize, &'static str)> {
        Self::HOOKS
            .iter()
            .filter_map(|hook| text.find(hook).map(|at| (at, *hook)))
            .min()
    }

    fn hook_failure(helper: &str, path: &Path, at: usize, hook: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        let message = format!(
            "The Lua Delegate {helper} has an OnStart, OnStop or OnEvent method. A Lua Tac Delegate is built with the service that needs it, so the page never runs those hooks."
        );
        let help = format!(
            "Move the hook from {portable} to the service that needs the Delegate, and let it call plain Delegate methods."
        );
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(help),
            source_span(&portable, at, at + hook.len()),
        ))
    }

    fn files(snapshot_root: &Path, folder: &Path, depth: usize) -> Vec<(String, PathBuf, String)> {
        std::fs::read_dir(snapshot_root.join(folder))
            .into_iter()
            .flatten()
            .flatten()
            .flat_map(|entry| Self::files_at(snapshot_root, &folder.join(entry.file_name()), depth))
            .collect()
    }

    fn files_at(
        snapshot_root: &Path,
        relative: &Path,
        depth: usize,
    ) -> Vec<(String, PathBuf, String)> {
        let full = snapshot_root.join(relative);
        let folder = std::fs::symlink_metadata(&full).is_ok_and(|metadata| metadata.is_dir());
        if [folder, depth < Self::MOST_DEPTH] == [true, true] {
            return Self::files(snapshot_root, relative, depth + 1);
        }
        Self::helper_file(&full, relative).into_iter().collect()
    }

    fn helper_file(full: &Path, relative: &Path) -> Option<(String, PathBuf, String)> {
        relative
            .extension()
            .filter(|extension| *extension == Self::EXTENSION)?;
        let name = relative.file_stem()?.to_str()?.to_owned();
        let text = std::fs::read_to_string(full).ok()?;
        Some((
            name,
            relative.to_owned(),
            crate::without_bom(&text).to_owned(),
        ))
    }
}
