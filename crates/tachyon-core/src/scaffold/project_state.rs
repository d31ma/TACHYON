use super::{
    AppFeature, AppTarget, Choice, ProjectChoices, RealmChoice, ScaffoldFailure, TacLanguage,
    YonLanguage,
};
use crate::Failure;
use std::path::Path;

pub(crate) struct ProjectState;

impl ProjectState {
    const CLIENT: &'static str = "client";
    const SERVER: &'static str = "server";
    const PACKAGE: &'static str = "package.json";
    const SHELL_SERVICE: &'static str = "client/services/ShellService.js";
    const HOME_CONTROLLER: &'static str = "server/controllers/HomeController.js";

    pub(crate) fn detect(root: &Path) -> Result<ProjectChoices, Failure> {
        let realm = Self::realm(root)?;
        Ok(ProjectChoices {
            realm,
            tac: Self::tac(root),
            yon: Self::yon(root),
            targets: Self::targets(root, realm),
            features: Self::features(root),
        })
    }

    fn realm(root: &Path) -> Result<RealmChoice, Failure> {
        let client = root.join(Self::CLIENT).is_dir();
        let server = root.join(Self::SERVER).is_dir();
        match (client, server) {
            (true, true) => Ok(RealmChoice::Both),
            (true, false) => Ok(RealmChoice::Client),
            (false, true) => Ok(RealmChoice::Server),
            (false, false) => Err(ScaffoldFailure::target(format!(
                "'{}' has no client/ or server/ folder, so it is not a Tachyon project.",
                root.display()
            ))),
        }
    }

    fn tac(root: &Path) -> Vec<TacLanguage> {
        TacLanguage::ALL
            .iter()
            .copied()
            .filter(|language| root.join(Self::tac_marker(*language)).is_file())
            .collect()
    }

    fn yon(root: &Path) -> Vec<YonLanguage> {
        YonLanguage::ALL
            .iter()
            .copied()
            .filter(|language| root.join(Self::yon_marker(*language)).is_file())
            .collect()
    }

    fn tac_marker(language: TacLanguage) -> String {
        Self::first_file_or(language.files(), Self::SHELL_SERVICE)
    }

    fn yon_marker(language: YonLanguage) -> String {
        Self::first_file_or(language.files(), Self::HOME_CONTROLLER)
    }

    fn first_file_or(files: Vec<(String, String)>, fallback: &str) -> String {
        files
            .into_iter()
            .next()
            .map_or_else(|| String::from(fallback), |(path, _)| path)
    }

    fn targets(root: &Path, realm: RealmChoice) -> Vec<AppTarget> {
        let package = std::fs::read_to_string(root.join(Self::PACKAGE)).unwrap_or_default();
        AppTarget::ALL
            .iter()
            .copied()
            .filter(|target| Self::has_target(&package, *target, realm))
            .collect()
    }

    fn has_target(package: &str, target: AppTarget, realm: RealmChoice) -> bool {
        let script = format!("\"bundle:{}\"", target.flag_name());
        let web = [target == AppTarget::Web, realm.has_client()] == [true, true];
        web || package.contains(&script)
    }

    fn features(root: &Path) -> Vec<AppFeature> {
        AppFeature::ALL
            .iter()
            .copied()
            .filter(|feature| Self::has_feature(root, *feature))
            .collect()
    }

    fn has_feature(root: &Path, feature: AppFeature) -> bool {
        let mut files = feature.files(false);
        files.extend(feature.files(true));
        files.iter().any(|(path, _)| root.join(path).is_file())
    }
}
