use super::{
    AdditionResult, AppFeature, AppName, AppTarget, Choice, ChoiceError, ChoiceList, ClientFiles,
    FeatureFiles, PageFile, ProjectChoices, ProjectFiles, ProjectState, ProjectWriter, RealmChoice,
    ScaffoldFailure, ServerFiles, TacLanguage, YonLanguage,
};
use crate::Failure;
use std::collections::BTreeMap;
use std::path::Path;

pub struct ProjectAddition;

impl ProjectAddition {
    const MANIFEST: &'static str = "client/manifest.json";
    const PACKAGE: &'static str = "package.json";
    const SCRIPTS: &'static str = "\"scripts\": {\n";

    pub fn language(
        root: &Path,
        realm: RealmChoice,
        language: &str,
    ) -> Result<AdditionResult, Failure> {
        Self::language_in(root, realm, language)
    }

    pub fn tac_language(root: &Path, language: TacLanguage) -> Result<AdditionResult, Failure> {
        let mut choices = ProjectState::detect(root)?;
        choices.realm = Self::with_client(choices.realm);
        choices.tac.push(language);
        let app_name = Self::app_name(root);
        let javascript_name = AppName::javascript_literal(&app_name);
        let owned = ClientFiles::language_files(language, &javascript_name);
        let tags: Vec<String> = ClientFiles::greeting_tag(language).into_iter().collect();
        Self::add(root, choices, &owned, &tags)
    }

    pub fn yon_language(root: &Path, language: YonLanguage) -> Result<AdditionResult, Failure> {
        let mut choices = ProjectState::detect(root)?;
        choices.realm = Self::with_server(choices.realm);
        choices.yon.push(language);
        let owned = ServerFiles::language_files(language);
        Self::add(root, choices, &owned, &[])
    }

    pub fn feature(root: &Path, feature: AppFeature) -> Result<AdditionResult, Failure> {
        let mut choices = ProjectState::detect(root)?;
        choices.features.push(feature);
        let choices = choices.checked()?;
        let owned = FeatureFiles::for_features(&[feature], &choices);
        let tags: Vec<String> = ClientFiles::feature_tag(feature)
            .filter(|_| choices.realm.has_client())
            .into_iter()
            .collect();
        let mut result = Self::add(root, choices, &owned, &tags)?;
        result.notes.extend(Self::add_datasource(root, feature)?);
        Ok(result)
    }

    pub fn target(root: &Path, target: AppTarget) -> Result<AdditionResult, Failure> {
        let mut choices = ProjectState::detect(root)?;
        choices.targets.push(target);
        let choices = choices.checked()?;
        let notes = Self::add_target_script(root, target)?;
        Ok(AdditionResult {
            choices,
            written: Vec::new(),
            notes,
        })
    }

    fn language_in(
        root: &Path,
        realm: RealmChoice,
        language: &str,
    ) -> Result<AdditionResult, Failure> {
        match realm {
            RealmChoice::Client => {
                Self::tac_language(root, ChoiceList::one(language, "Tac language")?)
            }
            RealmChoice::Server => {
                Self::yon_language(root, ChoiceList::one(language, "Yon language")?)
            }
            RealmChoice::Both => Err(ChoiceError::conflict(
                String::from("A language belongs to one realm."),
                "Write client for a Tac language or server for a Yon language, for example: ty add language server py",
            )),
        }
    }

    fn add(
        root: &Path,
        choices: ProjectChoices,
        owned: &[(String, String)],
        tags: &[String],
    ) -> Result<AdditionResult, Failure> {
        let choices = choices.checked()?;
        Self::refuse_existing(root, owned)?;
        let mut files: BTreeMap<String, String> = Self::missing_base_files(root, &choices);
        files.extend(owned.iter().cloned());
        let written: Vec<String> = files.keys().cloned().collect();
        ProjectWriter::write_all(root, &files, root)?;
        let notes = tags
            .iter()
            .filter_map(|tag| Self::place_component(root, tag))
            .collect();
        Ok(AdditionResult {
            choices,
            written,
            notes,
        })
    }

    fn missing_base_files(root: &Path, choices: &ProjectChoices) -> BTreeMap<String, String> {
        let base_choices = ProjectChoices {
            tac: Vec::new(),
            yon: Vec::new(),
            features: Vec::new(),
            ..choices.clone()
        };
        let mut files = ProjectFiles::for_choices(&Self::app_name(root), &base_choices);
        files.retain(|path, _| !root.join(path).exists());
        files
    }

    fn refuse_existing(root: &Path, owned: &[(String, String)]) -> Result<(), Failure> {
        let existing = owned
            .iter()
            .map(|(path, _)| root.join(path))
            .find(|path| path.exists());
        existing.map_or(Ok(()), |path| Err(ScaffoldFailure::existing_file(&path)))
    }

    fn place_component(root: &Path, tag: &str) -> Option<String> {
        let page_path = root.join(PageFile::PATH);
        let page = std::fs::read_to_string(&page_path).unwrap_or_default();
        let changed = PageFile::with_component(&page, tag);
        Self::write_page(&page_path, changed, tag)
    }

    fn write_page(page_path: &Path, changed: Option<String>, tag: &str) -> Option<String> {
        let written = changed.is_some_and(|text| std::fs::write(page_path, text).is_ok());
        (!written).then(|| format!("Place {} in one of your pages.", PageFile::line(tag).trim()))
    }

    fn add_datasource(root: &Path, feature: AppFeature) -> Result<Option<String>, Failure> {
        if feature != AppFeature::Storage {
            return Ok(None);
        }
        Self::write_datasource(root)
    }

    fn write_datasource(root: &Path) -> Result<Option<String>, Failure> {
        let path = root.join(ServerFiles::APPLICATION);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let mut settings: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({}));
        let datasource = serde_json::json!({ "driver": "sqlite", "url": "sqlite::memory:" });
        let added = Self::insert_missing(&mut settings, "datasource", datasource);
        let pretty = serde_json::to_string_pretty(&settings).unwrap_or_default();
        ScaffoldFailure::checked(std::fs::write(&path, format!("{pretty}\n")), &path)?;
        Ok(added.then(|| {
            String::from("server/application.json now names the in-memory SQLite datasource.")
        }))
    }

    fn insert_missing(
        settings: &mut serde_json::Value,
        key: &str,
        value: serde_json::Value,
    ) -> bool {
        let Some(object) = settings.as_object_mut() else {
            return false;
        };
        let missing = !object.contains_key(key);
        object.entry(key).or_insert(value);
        missing
    }

    fn add_target_script(root: &Path, target: AppTarget) -> Result<Vec<String>, Failure> {
        let path = root.join(Self::PACKAGE);
        let package = std::fs::read_to_string(&path).unwrap_or_default();
        let changed = Self::with_script(&package, target);
        changed.map_or_else(
            || {
                Ok(vec![format!(
                    "Build this target with: ty bundle --target {}",
                    target.flag_name()
                )])
            },
            |text| {
                ScaffoldFailure::checked(std::fs::write(&path, text), &path).map(|()| Vec::new())
            },
        )
    }

    fn with_script(package: &str, target: AppTarget) -> Option<String> {
        let script = target.bundle_script();
        let place = package.find(Self::SCRIPTS)? + Self::SCRIPTS.len();
        let needed = [target.is_native(), !package.contains(script.trim())] == [true, true];
        needed.then(|| format!("{}{script}{}", &package[..place], &package[place..]))
    }

    fn app_name(root: &Path) -> String {
        let manifest = std::fs::read_to_string(root.join(Self::MANIFEST)).unwrap_or_default();
        let parsed: serde_json::Value = serde_json::from_str(&manifest).unwrap_or_default();
        parsed["name"].as_str().map_or_else(
            || AppName::validated(None, root).unwrap_or_default(),
            String::from,
        )
    }

    fn with_client(realm: RealmChoice) -> RealmChoice {
        if realm.has_client() {
            return realm;
        }
        RealmChoice::Both
    }

    fn with_server(realm: RealmChoice) -> RealmChoice {
        if realm.has_server() {
            return realm;
        }
        RealmChoice::Both
    }
}
