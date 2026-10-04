use super::{CppFailure, CppFolder, CppLowering, CppModel, CppPrelude, CppService};
use crate::Failure;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(crate) struct CppProgram {
    models: Vec<CppModel>,
    services: Vec<CppService>,
    registered: Vec<(String, String)>,
}

impl CppProgram {
    const MODELS: &'static str = "client/models";
    const SERVICES: &'static str = "client/services";
    const GENERATED: &'static str = "#line 1 \"tachyon-generated.cpp\"\n";
    const CIRCLE: &'static str =
        "C++ services need each other in a circle; break the circle with an event";

    pub(crate) fn read(root: &Path, chosen: &[(String, String, PathBuf)]) -> Result<Self, Failure> {
        let models = Self::models(root)?;
        let available = Self::available_services(root)?;
        let mut program = Self {
            models,
            services: Vec::new(),
            registered: Vec::new(),
        };
        let mut visiting = BTreeSet::new();
        for (instance, class, path) in chosen {
            program.registered.push((instance.clone(), class.clone()));
            program.add_service(root, path, &available, &mut visiting)?;
        }
        Ok(program)
    }

    fn models(root: &Path) -> Result<Vec<CppModel>, Failure> {
        let mut models = Vec::new();
        for path in CppFolder::files(root, Self::MODELS)? {
            let (relative, source) = CppFolder::read(root, &path)?;
            models.push(CppModel::read(&relative, &source)?);
        }
        Ok(Self::ordered_models(models))
    }

    fn ordered_models(mut waiting: Vec<CppModel>) -> Vec<CppModel> {
        let mut ordered: Vec<CppModel> = Vec::new();
        while !waiting.is_empty() {
            let names: Vec<String> = waiting
                .iter()
                .map(|model| model.name().to_owned())
                .collect();
            let ready = waiting
                .iter()
                .position(|model| !Self::mentions_any(model, &names))
                .unwrap_or(0);
            ordered.push(waiting.remove(ready));
        }
        ordered
    }

    fn mentions_any(model: &CppModel, names: &[String]) -> bool {
        let types = model.field_types().join(" ");
        let words: Vec<&str> = types
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .collect();
        names
            .iter()
            .any(|name| name != model.name() && words.contains(&name.as_str()))
    }

    fn available_services(root: &Path) -> Result<BTreeMap<String, PathBuf>, Failure> {
        let mut available = BTreeMap::new();
        for path in CppFolder::files(root, Self::SERVICES)? {
            let stem = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            available.insert(stem, path);
        }
        Ok(available)
    }

    fn add_service(
        &mut self,
        root: &Path,
        path: &Path,
        available: &BTreeMap<String, PathBuf>,
        visiting: &mut BTreeSet<PathBuf>,
    ) -> Result<(), Failure> {
        let added = self
            .services
            .iter()
            .any(|service| root.join(&service.path) == path);
        if added {
            return Ok(());
        }
        let (relative, source) = CppFolder::read(root, path)?;
        if !visiting.insert(relative.clone()) {
            return Err(CppFailure::unreadable(&relative, 0, Self::CIRCLE));
        }
        let service = CppService::read(&relative, &source)?;
        for (field, needed) in service.needs() {
            let Some(needed_path) = available.get(&needed) else {
                let at = service
                    .class()
                    .members
                    .iter()
                    .find(|member| member.name == field)
                    .map_or(0, |member| member.name_at);
                return Err(CppFailure::pending(
                    &relative,
                    at,
                    &format!(
                        "A C++ [[Needs]] field names another C++ service in client/services; there is no client/services/{needed}.hpp. Other C++ roles come later (ADR 0098)"
                    ),
                ));
            };
            self.add_service(root, needed_path, available, visiting)?;
        }
        self.services.push(service);
        Ok(())
    }

    pub(crate) fn every_service(root: &Path) -> Result<Self, Failure> {
        let available = Self::available_services(root)?;
        let mut program = Self {
            models: Self::models(root)?,
            services: Vec::new(),
            registered: Vec::new(),
        };
        let mut visiting = BTreeSet::new();
        for path in available.values() {
            program.add_service(root, path, &available, &mut visiting)?;
        }
        Ok(program)
    }

    pub(crate) fn web_source(&self) -> String {
        format!(
            "{}{}{}{}{}{}{}{}",
            CppPrelude::CORE,
            CppPrelude::WEB_HOST,
            CppPrelude::PAGE,
            CppPrelude::SERVICES,
            self.authored(),
            self.bindings(),
            self.registration(),
            CppPrelude::WEB_EXPORTS
        )
    }

    pub(crate) fn test_source(&self, units: &str) -> String {
        format!(
            "{}{}{}{}{units}",
            CppPrelude::CORE,
            CppPrelude::TEST,
            CppPrelude::SERVICES,
            self.authored(),
        )
    }

    pub(crate) fn located(path: &Path, lowered: &str) -> String {
        format!("#line 1 \"{}\"\n{lowered}\n", Self::portable(path))
    }

    fn authored(&self) -> String {
        let mut text = String::new();
        for model in &self.models {
            text.push_str(&Self::located(&model.path, &model.lowered()));
        }
        text.push_str(Self::GENERATED);
        for model in &self.models {
            text.push_str(&model.codec());
        }
        for service in &self.services {
            text.push_str(&Self::located(
                &service.path,
                &CppLowering::service(service),
            ));
        }
        text
    }

    fn bindings(&self) -> String {
        let mut text = String::from(Self::GENERATED);
        for service in &self.services {
            text.push_str(&service.binding());
        }
        text
    }

    fn registration(&self) -> String {
        let mut entries = String::new();
        for (instance, class) in &self.registered {
            let _ = writeln!(
                entries,
                "    services[\"{instance}\"] = TachyonBinding{class}::type();"
            );
        }
        format!(
            "std::map<std::string, tachyon::TacServiceType> tachyon::TacProgram::services() {{\n    std::map<std::string, tachyon::TacServiceType> services;\n{entries}    return services;\n}}\n"
        )
    }

    fn portable(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }
}
