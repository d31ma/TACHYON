use super::{AppName, ProjectChoices, ProjectFiles, ProjectWriter, ScaffoldResult, TargetFolder};
use crate::Failure;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub struct Scaffold;

impl Scaffold {
    pub const DEFAULT_ENV: &'static str = super::ServerFiles::DEFAULT_ENV;

    pub fn create(
        target: impl AsRef<Path>,
        app_name: Option<&str>,
    ) -> Result<ScaffoldResult, Failure> {
        Self::create_with(target, app_name, &ProjectChoices::defaults())
    }

    pub fn create_with(
        target: impl AsRef<Path>,
        app_name: Option<&str>,
        choices: &ProjectChoices,
    ) -> Result<ScaffoldResult, Failure> {
        let target = TargetFolder::absolute(target.as_ref())?;
        TargetFolder::check_empty(&target)?;
        let app_name = AppName::validated(app_name, &target)?;
        let files = ProjectFiles::for_choices(&app_name, choices);
        ProjectWriter::publish(&target, &files)?;
        let root = TargetFolder::canonical(&target)?;
        Ok(ScaffoldResult { root, app_name })
    }
}
