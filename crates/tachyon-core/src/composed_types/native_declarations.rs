use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) struct NativeDeclarations {
    language: &'static str,
    extension: &'static str,
    services: BTreeMap<PathBuf, String>,
    models: BTreeMap<PathBuf, String>,
}

impl NativeDeclarations {
    pub(super) fn new(language: &'static str, extension: &'static str) -> Self {
        Self {
            language,
            extension,
            services: BTreeMap::new(),
            models: BTreeMap::new(),
        }
    }

    pub(super) fn accepts(&self, path: &Path) -> bool {
        self.accepts_file(path)
    }

    pub(super) fn add(&mut self, path: &Path, source: Option<String>) {
        let destination = self.destination(path);
        Self::add_present(destination, path, source);
    }

    pub(super) fn lines(
        &self,
        script_names: &[String],
        peers: &BTreeSet<String>,
    ) -> Result<String, Failure> {
        self.require_unique_services()?;
        let selected = self.selected(peers);
        selected.require_distinct_names(script_names)?;
        selected.render(
            &Self::files(&selected.services),
            &Self::files(&selected.models),
        )
    }

    fn selected(&self, peers: &BTreeSet<String>) -> Self {
        let services = self
            .services
            .iter()
            .filter(|(path, _)| Self::is_selected(path, peers))
            .map(|(path, source)| (path.clone(), source.clone()))
            .collect();
        Self {
            language: self.language,
            extension: self.extension,
            services,
            models: self.models.clone(),
        }
    }

    fn is_selected(path: &Path, peers: &BTreeSet<String>) -> bool {
        path.file_stem()
            .and_then(|name| name.to_str())
            .is_none_or(|name| !peers.contains(name))
    }

    fn accepts_file(&self, path: &Path) -> bool {
        self.is_language(path) && Self::is_role_folder(path)
    }

    fn is_language(&self, path: &Path) -> bool {
        path.extension().and_then(|name| name.to_str()) == Some(self.extension)
    }

    fn is_role_folder(path: &Path) -> bool {
        ["client/services", "client/models"]
            .iter()
            .any(|folder| path.starts_with(folder))
    }

    fn add_present(
        destination: &mut BTreeMap<PathBuf, String>,
        path: &Path,
        source: Option<String>,
    ) {
        if let Some(source) = source {
            destination.insert(path.to_path_buf(), source);
        }
    }

    fn destination(&mut self, path: &Path) -> &mut BTreeMap<PathBuf, String> {
        if path.starts_with("client/models") {
            &mut self.models
        } else {
            &mut self.services
        }
    }

    fn files(files: &BTreeMap<PathBuf, String>) -> Vec<(PathBuf, String)> {
        files
            .iter()
            .map(|(path, source)| (path.clone(), source.clone()))
            .collect()
    }

    fn require_distinct_names(&self, script_names: &[String]) -> Result<(), Failure> {
        self.services
            .keys()
            .try_for_each(|path| self.require_unique(path, script_names))?;
        self.require_model_namespace(script_names)
    }

    fn require_unique(&self, path: &Path, script_names: &[String]) -> Result<(), Failure> {
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if script_names.iter().any(|other| other == name) {
            return Err(self.collision(path, name));
        }
        Ok(())
    }

    fn require_unique_services(&self) -> Result<(), Failure> {
        let mut names = std::collections::BTreeSet::new();
        self.services
            .keys()
            .try_for_each(|path| self.insert_unique_name(path, &mut names))
    }

    fn insert_unique_name(
        &self,
        path: &Path,
        names: &mut std::collections::BTreeSet<String>,
    ) -> Result<(), Failure> {
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if !names.insert(name.to_owned()) {
            return Err(self.collision(path, name));
        }
        Ok(())
    }

    fn require_model_namespace(&self, script_names: &[String]) -> Result<(), Failure> {
        let path = self.models.keys().next();
        let duplicate = script_names.iter().any(|name| name == &self.namespace());
        match (path, duplicate, self.services.is_empty()) {
            (Some(path), true, false) => Err(self.collision(path, &self.namespace())),
            _ => Ok(()),
        }
    }

    pub(super) fn names(&self, peers: &BTreeSet<String>) -> Vec<String> {
        self.service_names(peers)
    }

    fn service_names(&self, peers: &BTreeSet<String>) -> Vec<String> {
        self.services
            .keys()
            .filter(|path| Self::is_selected(path, peers))
            .filter_map(|path| path.file_stem()?.to_str().map(String::from))
            .collect()
    }

    fn namespace(&self) -> String {
        format!("__Tachyon{}Models", self.language)
    }

    fn render(
        &self,
        services: &[(PathBuf, String)],
        models: &[(PathBuf, String)],
    ) -> Result<String, Failure> {
        match self.extension {
            "java" => crate::native::JavaWebServices::composed_types(services, models),
            "kt" => crate::native::KotlinWebServices::composed_types(services, models),
            _ => Ok(String::new()),
        }
    }

    fn collision(&self, path: &Path, name: &str) -> Failure {
        Failure::one(diagnostic(
            1410,
            format!(
                "The composed {} browser type '{name}' conflicts with an existing composed name.",
                self.language
            ),
            Some(String::from(
                "Give each composed service and model type a distinct name across languages.",
            )),
            source_span(&path.to_string_lossy(), 0, 1),
        ))
    }
}
