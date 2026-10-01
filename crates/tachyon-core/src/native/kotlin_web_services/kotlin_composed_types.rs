use super::kotlin_type_members::KotlinTypeMembers;
use super::kotlin_type_models::KotlinTypeModels;
use crate::Failure;
use std::path::{Path, PathBuf};

pub(super) struct KotlinComposedTypes<'a> {
    models: KotlinTypeModels,
    files: &'a [(PathBuf, String)],
}

impl<'a> KotlinComposedTypes<'a> {
    pub(super) fn render(
        services: &[(PathBuf, String)],
        files: &'a [(PathBuf, String)],
    ) -> Result<String, Failure> {
        Self::read(files, !services.is_empty())?
            .map_or(Ok(String::new()), |types| types.program(services))
    }

    fn read(files: &'a [(PathBuf, String)], needed: bool) -> Result<Option<Self>, Failure> {
        if !needed {
            return Ok(None);
        }
        Ok(Some(Self {
            models: KotlinTypeModels::read(files)?,
            files,
        }))
    }

    fn program(&self, services: &[(PathBuf, String)]) -> Result<String, Failure> {
        super::super::native_type_program::NativeTypeProgram::render(
            services,
            self.models.lines(),
            |path, source| self.service(path, source),
        )
    }

    fn service(&self, path: &Path, source: &str) -> Result<String, Failure> {
        let class = self.service_class(path, source)?;
        let roles = crate::behavior::ServiceRoles::read(path, source)?;
        Self::validate_annotations(path, &roles.source, self.files)?;
        let members = crate::companion::KotlinMembers::read(path, source)?;
        let required = Self::required_properties(path, source, &class, !roles.required.is_empty());
        let values = self.models.values();
        let lines = KotlinTypeMembers::lines(path, &members, &values)?;
        let properties = KotlinTypeMembers::constructor(path, &members, &required, &values)?;
        Ok(format!(
            "  interface {class} {{\n{lines}  }}\n  const {class}: {{ new ({properties}): {class}; }};\n"
        ))
    }

    fn service_class(&self, path: &Path, source: &str) -> Result<String, Failure> {
        crate::annotation_layout::FolderRoleCheck::check(path, source)?;
        let class = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        self.models.require_service_name(path, class)?;
        crate::companion::KotlinHeader::require_browser_constructor(path, source, class)?;
        Ok(class.to_owned())
    }

    fn validate_annotations(
        path: &Path,
        source: &str,
        files: &[(PathBuf, String)],
    ) -> Result<(), Failure> {
        let assembled =
            crate::annotation_modules::Module::with_models(path, source, files.to_vec())?;
        crate::behavior::lower(path, &assembled, "tac").map(|_| ())
    }

    fn required_properties(path: &Path, source: &str, class: &str, needed: bool) -> Vec<String> {
        if !needed {
            return Vec::new();
        }
        let mut providers = crate::web_api_clients::WebApiClients::TYPE_NAMES
            .iter()
            .map(|name| String::from(*name))
            .collect::<std::collections::BTreeSet<_>>();
        providers.extend([
            String::from("ApplicationEventPublisher"),
            String::from("DocumentTitle"),
        ]);
        crate::template::RequiredFields::props(path, source, class, &providers)
    }
}
