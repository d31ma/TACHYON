use super::java_class_reader::JavaClassReader;
use super::java_preference_constructor::JavaPreferenceConstructor;
use super::java_type_members::JavaTypeMembers;
use super::java_type_models::JavaTypeModels;
use crate::Failure;
use std::path::{Path, PathBuf};

pub(super) struct JavaComposedTypes;

impl JavaComposedTypes {
    pub(super) fn render(
        services: &[(PathBuf, String)],
        files: &[(PathBuf, String)],
    ) -> Result<String, Failure> {
        Self::nonempty(services).map_or(Ok(String::new()), |services| {
            Self::render_services(services, files)
        })
    }

    fn nonempty(services: &[(PathBuf, String)]) -> Option<&[(PathBuf, String)]> {
        (!services.is_empty()).then_some(services)
    }

    fn render_services(
        services: &[(PathBuf, String)],
        files: &[(PathBuf, String)],
    ) -> Result<String, Failure> {
        let models = JavaTypeModels::read(files)?;
        let mut text = models.lines();
        for (path, source) in services {
            text.push_str(&Self::service(path, source, &models, files)?);
        }
        Ok(text)
    }

    fn service(
        path: &Path,
        source: &str,
        models: &JavaTypeModels,
        files: &[(PathBuf, String)],
    ) -> Result<String, Failure> {
        crate::annotation_layout::FolderRoleCheck::check(path, source)?;
        let class = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        models.require_service_name(path, class)?;
        JavaPreferenceConstructor::read(path, source, class)?;
        let roles = crate::behavior::ServiceRoles::read(path, source)?;
        let required = Self::required_properties(path, source, class, !roles.required.is_empty());
        let assembled =
            crate::annotation_modules::Module::with_models(path, &roles.source, files.to_vec())?;
        let lowered = crate::behavior::lower(path, &assembled, "tac")?.unwrap_or(assembled);
        let plain = models.without_models(path, &lowered)?;
        let shown = path.to_string_lossy();
        let values = models.values();
        let injected = crate::template::RequiredFields::injected_fields(
            path,
            source,
            class,
            &Self::providers(),
        );
        let members = JavaClassReader::new(&shown, &plain)
            .members_without_fields(class, &values, &injected)?;
        JavaTypeMembers::require_properties(path, &members, &required)?;
        let lines = JavaTypeMembers::lines(&members, &values);
        let properties = JavaTypeMembers::constructor(&members, &required, &values);
        Ok(format!(
            "  interface {class} {{\n{lines}  }}\n  const {class}: {{ new ({properties}): {class}; }};\n"
        ))
    }
    fn required_properties(path: &Path, source: &str, class: &str, needed: bool) -> Vec<String> {
        if !needed {
            return Vec::new();
        }
        crate::template::RequiredFields::props(path, source, class, &Self::providers())
    }

    fn providers() -> std::collections::BTreeSet<String> {
        [
            String::from("PreferenceStrings"),
            String::from("ApplicationEventPublisher"),
        ]
        .into_iter()
        .collect()
    }
}
