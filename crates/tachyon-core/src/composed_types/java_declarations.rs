use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) struct JavaDeclarations {
    services: BTreeMap<PathBuf, String>,
    models: BTreeMap<PathBuf, String>,
}

impl JavaDeclarations {
    pub(super) fn new() -> Self {
        Self {
            services: BTreeMap::new(),
            models: BTreeMap::new(),
        }
    }

    pub(super) fn accepts(path: &Path) -> bool {
        Self::accepts_file(path)
    }

    pub(super) fn add(&mut self, path: &Path, source: Option<String>) {
        let destination = self.destination(path);
        Self::add_present(destination, path, source);
    }

    pub(super) fn lines(&self, script_names: &[String]) -> Result<String, Failure> {
        self.require_distinct_names(script_names)?;
        crate::native::JavaWebServices::composed_types(
            &Self::files(&self.services),
            &Self::files(&self.models),
        )
    }

    fn accepts_file(path: &Path) -> bool {
        Self::is_java(path) && Self::is_role_folder(path)
    }

    fn is_java(path: &Path) -> bool {
        path.extension().and_then(|name| name.to_str()) == Some("java")
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
            .try_for_each(|path| Self::require_unique(path, script_names))?;
        self.require_unique_services()?;
        self.require_model_namespace(script_names)
    }

    fn require_unique(path: &Path, script_names: &[String]) -> Result<(), Failure> {
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if script_names.iter().any(|other| other == name) {
            return Err(Self::collision(path, name));
        }
        Ok(())
    }

    fn require_unique_services(&self) -> Result<(), Failure> {
        let mut names = std::collections::BTreeSet::new();
        self.services
            .keys()
            .try_for_each(|path| Self::insert_unique_name(path, &mut names))
    }

    fn insert_unique_name(
        path: &Path,
        names: &mut std::collections::BTreeSet<String>,
    ) -> Result<(), Failure> {
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if !names.insert(name.to_owned()) {
            return Err(Self::collision(path, name));
        }
        Ok(())
    }

    fn require_model_namespace(&self, script_names: &[String]) -> Result<(), Failure> {
        let path = self.models.keys().next();
        let duplicate = script_names
            .iter()
            .any(|name| name == "__TachyonJavaModels");
        match (path, duplicate, self.services.is_empty()) {
            (Some(path), true, false) => Err(Self::collision(path, "__TachyonJavaModels")),
            _ => Ok(()),
        }
    }

    fn collision(path: &Path, name: &str) -> Failure {
        Failure::one(diagnostic(
            1410,
            format!(
                "The composed Java browser type '{name}' conflicts with an existing composed name."
            ),
            Some(String::from(
                "Give each composed service and model type a distinct name across languages.",
            )),
            source_span(&path.to_string_lossy(), 0, 1),
        ))
    }
}
