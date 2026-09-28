use super::RoleFiles;
use crate::Failure;
use crate::failure::diagnostic;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecordedCalls {
    realms: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    languages: BTreeMap<String, BTreeMap<String, bool>>,
    #[serde(default)]
    successful: bool,
}

impl RecordedCalls {
    const FOLDER: [&'static str; 2] = [".tachyon", "coverage"];
    const FILE_NAME: &'static str = "calls.json";
    const WRITE_FAILURE: u16 = 2024;

    pub fn add(&mut self, realm: &str, case: &str, called: &[String]) {
        let calls = self.realms.entry(realm.to_owned()).or_default();
        Self::keep_each(calls, called, case);
    }

    pub fn add_unit(&mut self, realm: &str, language: &str, ran: bool) {
        let languages = self.languages.entry(realm.to_owned()).or_default();
        let any_ran = languages.entry(language.to_owned()).or_default();
        *any_ran |= ran;
    }

    pub fn set_successful(&mut self, successful: bool) {
        self.successful = successful;
    }

    pub fn invalidate(project: &Path) -> Result<(), Failure> {
        let mut calls = Self::read(project).unwrap_or_default();
        calls.set_successful(false);
        calls.write(project)
    }

    #[must_use]
    pub fn could_not_run(&self, realm: &str, language: &str) -> bool {
        self.languages
            .get(realm)
            .and_then(|languages| languages.get(language))
            .is_some_and(|ran| !ran)
    }

    #[must_use]
    pub fn first_case(&self, realm: &str, key: &str) -> Option<&str> {
        self.realms
            .get(realm)
            .and_then(|calls| calls.get(key))
            .map(String::as_str)
    }

    #[must_use]
    pub fn read(project: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(Self::path(project)).ok()?;
        serde_json::from_str(&text).ok()
    }

    #[must_use]
    pub fn read_current(project: &Path) -> Option<Self> {
        let recorded = Self::recorded_time(project)?;
        let changed = RoleFiles::newest_change(project);
        Self::read(project)
            .filter(|calls| calls.successful)
            .filter(|_| changed.is_none_or(|changed| changed <= recorded))
    }

    pub fn write(&self, project: &Path) -> Result<(), Failure> {
        let path = Self::path(project);
        let text = serde_json::to_string_pretty(self).unwrap_or_default();
        Self::write_text(&path, &text).map_err(|error| Self::write_failure(&path, &error))
    }

    #[must_use]
    pub fn path(project: &Path) -> PathBuf {
        let folder: PathBuf = Self::FOLDER.iter().collect();
        project.join(folder).join(Self::FILE_NAME)
    }

    fn recorded_time(project: &Path) -> Option<SystemTime> {
        std::fs::metadata(Self::path(project))
            .and_then(|metadata| metadata.modified())
            .ok()
    }

    fn keep_each(calls: &mut BTreeMap<String, String>, called: &[String], case: &str) {
        for key in called {
            Self::keep_first(calls, key, case);
        }
    }

    fn keep_first(calls: &mut BTreeMap<String, String>, key: &str, case: &str) {
        calls
            .entry(key.to_owned())
            .or_insert_with(|| case.to_owned());
    }

    fn write_text(path: &Path, text: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(path.parent().unwrap_or(path))?;
        std::fs::write(path, text)
    }

    fn write_failure(path: &Path, error: &std::io::Error) -> Failure {
        Failure::one(diagnostic(
            Self::WRITE_FAILURE,
            format!(
                "Cannot write the recorded test calls to {}: {error}",
                path.display()
            ),
            Some(String::from(
                "Keep .tachyon/coverage a regular directory inside the project that ty can write.",
            )),
            None,
        ))
    }
}
