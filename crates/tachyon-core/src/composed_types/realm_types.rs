use super::composition_check::CompositionCheck;
use super::declaration::Declaration;
use super::exported_class::ExportedClass;
use super::model_declaration::ModelDeclaration;
use cap_std::fs::{Dir, DirEntry, Metadata};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

pub(super) struct RealmTypes<'a> {
    project: &'a Dir,
    realm: &'a str,
    entry_count: usize,
    declarations: BTreeMap<String, Declaration>,
    models: BTreeMap<String, ModelDeclaration>,
}

impl<'a> RealmTypes<'a> {
    const EXTENSIONS: [&'static str; 2] = ["js", "ts"];
    const MAX_SOURCE: u64 = 1024 * 1024;
    const MAX_ENTRIES: usize = 4096;
    const MAX_DEPTH: usize = 32;
    const MODEL_FOLDER: &'static str = "models";
    const MODEL_EXTENSION: &'static str = "js";

    pub(super) fn render(project: &'a Dir, realm: &'a str) -> String {
        let mut types = Self {
            project,
            realm,
            entry_count: 0,
            declarations: BTreeMap::new(),
            models: BTreeMap::new(),
        };
        types.search_role_folders();
        types.text()
    }

    fn search_role_folders(&mut self) {
        for folder in self.folders_to_search() {
            self.search(&folder, 0);
        }
    }

    fn folders_to_search(&self) -> Vec<PathBuf> {
        let realm = Path::new(self.realm);
        let mut folders: Vec<PathBuf> = crate::annotation_modules::role_folders()
            .into_iter()
            .filter(|folder| self.uses_role_folder(folder))
            .map(|folder| realm.join(folder))
            .collect();
        folders.extend(self.error_folder());
        folders.extend(self.role_type_folder());
        folders.push(realm.join("utils"));
        folders.push(realm.join("tests"));
        folders.push(realm.join(Self::MODEL_FOLDER));
        folders
    }

    fn uses_role_folder(&self, folder: &str) -> bool {
        folder != "controllers" || self.is_server()
    }

    fn error_folder(&self) -> Option<PathBuf> {
        self.is_server().then(|| PathBuf::from("server/errors"))
    }

    fn role_type_folder(&self) -> Option<PathBuf> {
        self.is_server().then(|| PathBuf::from("server/roles"))
    }

    fn is_server(&self) -> bool {
        self.realm == "server"
    }

    fn search(&mut self, directory: &Path, depth: usize) {
        for name in self.names_to_search(directory, depth) {
            self.visit(&directory.join(name), depth);
        }
    }

    fn visit(&mut self, path: &Path, depth: usize) {
        if Self::is_real_directory(self.project, path) {
            self.search(path, depth + 1);
        } else {
            self.add_file(path);
        }
    }

    fn names_to_search(&mut self, directory: &Path, depth: usize) -> Vec<String> {
        let names = self.names_within_limits(directory, depth);
        Self::without_entry_folder(names.unwrap_or_default())
    }

    fn names_within_limits(&mut self, directory: &Path, depth: usize) -> Option<Vec<String>> {
        let allowed = self.may_search(directory, depth);
        allowed.then(|| self.sorted_names(directory)).flatten()
    }

    fn may_search(&self, directory: &Path, depth: usize) -> bool {
        depth <= Self::MAX_DEPTH && Self::is_real_directory(self.project, directory)
    }

    fn without_entry_folder(names: Vec<String>) -> Vec<String> {
        if names
            .iter()
            .any(|name| crate::annotation_modules::is_entry_name(name))
        {
            return Vec::new();
        }
        names
    }

    fn sorted_names(&mut self, directory: &Path) -> Option<Vec<String>> {
        let entries = self.project.read_dir(directory).ok()?;
        let mut names = Vec::new();
        for entry in entries {
            self.count_entry()?;
            names.extend(Self::entry_name(entry));
        }
        names.sort();
        Some(names)
    }

    fn count_entry(&mut self) -> Option<()> {
        self.entry_count += 1;
        (self.entry_count <= Self::MAX_ENTRIES).then_some(())
    }

    fn entry_name(entry: std::io::Result<DirEntry>) -> Option<String> {
        entry.ok()?.file_name().into_string().ok()
    }

    fn add_file(&mut self, path: &Path) {
        let classes = self.composed_classes(path);
        self.add_declarations(path, classes);
        let models = self.model_names(path);
        self.add_models(path, models);
    }

    fn model_names(&self, path: &Path) -> Vec<String> {
        let source = self.javascript_model_source(path).unwrap_or_default();
        crate::model_layout::ModelBlocks::names(path, &source).unwrap_or_default()
    }

    fn javascript_model_source(&self, path: &Path) -> Option<String> {
        let model = self.is_javascript_model(path);
        model.then(|| self.read_source(path)).flatten()
    }

    fn is_javascript_model(&self, path: &Path) -> bool {
        let folder = Path::new(self.realm).join(Self::MODEL_FOLDER);
        let extension = path.extension().and_then(|value| value.to_str());
        path.starts_with(folder) && extension == Some(Self::MODEL_EXTENSION)
    }

    fn add_models(&mut self, path: &Path, names: Vec<String>) {
        let portable = Self::portable(path);
        for name in names {
            self.models
                .entry(name)
                .or_insert_with(|| ModelDeclaration::new(portable.clone()));
        }
    }

    fn composed_classes(&self, path: &Path) -> Vec<ExportedClass> {
        self.read_source(path)
            .filter(|source| CompositionCheck::is_composed(path, source))
            .map(|source| ExportedClass::find(path, &source))
            .unwrap_or_default()
    }

    fn add_declarations(&mut self, path: &Path, classes: Vec<ExportedClass>) {
        let portable = Self::portable(path);
        for class in classes {
            self.declarations
                .entry(class.name)
                .or_insert_with(|| Declaration::new(portable.clone(), class.member));
        }
    }

    fn read_source(&self, path: &Path) -> Option<String> {
        Self::source_extension(path)?;
        self.small_file_size(path)?;
        self.project.read_to_string(path).ok()
    }

    fn source_extension(path: &Path) -> Option<&str> {
        path.extension()?
            .to_str()
            .filter(|extension| Self::EXTENSIONS.contains(extension))
    }

    fn small_file_size(&self, path: &Path) -> Option<u64> {
        let metadata = self.project.symlink_metadata(path).ok()?;
        Self::is_small_file(&metadata).then_some(metadata.len())
    }

    fn is_small_file(metadata: &Metadata) -> bool {
        metadata.is_file() && metadata.len() <= Self::MAX_SOURCE
    }

    fn text(&self) -> String {
        let mut text = String::from("export {};\n");
        text.push_str(&self.global_block());
        text
    }

    fn global_block(&self) -> String {
        if self.declarations.is_empty() && self.models.is_empty() {
            return String::new();
        }
        format!("\ndeclare global {{\n{}}}\n", self.declaration_lines())
    }

    fn declaration_lines(&self) -> String {
        let classes: String = self
            .declarations
            .iter()
            .map(|(name, declaration)| declaration.lines(name))
            .collect();
        let models: String = self
            .models
            .iter()
            .map(|(name, model)| model.line(name))
            .collect();
        format!("{classes}{models}")
    }

    fn is_real_directory(project: &Dir, path: &Path) -> bool {
        project
            .symlink_metadata(path)
            .is_ok_and(|metadata| metadata.is_dir())
    }

    fn portable(path: &Path) -> String {
        let parts: Vec<Cow<'_, str>> = path.components().filter_map(Self::normal_part).collect();
        parts.join("/")
    }

    fn normal_part(component: Component<'_>) -> Option<Cow<'_, str>> {
        match component {
            Component::Normal(part) => Some(part.to_string_lossy()),
            _ => None,
        }
    }
}
