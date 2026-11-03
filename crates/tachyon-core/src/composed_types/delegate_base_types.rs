use super::declaration::Declaration;
use super::exported_class::ExportedClass;
use crate::delegate_forwards::DelegateForwards;
use std::path::{Path, PathBuf};

pub(super) struct DelegateBaseTypes {
    services: Vec<(PathBuf, String)>,
    delegates: Vec<(String, Declaration, PathBuf, String)>,
}

impl DelegateBaseTypes {
    const SERVICE_FOLDER: &'static str = "services";
    const DELEGATE_FOLDER: &'static str = "delegates";
    const HIDDEN_STARTS: [&'static str; 5] = ["_", "#", "private ", "protected ", "static "];

    pub(super) fn new() -> Self {
        Self {
            services: Vec::new(),
            delegates: Vec::new(),
        }
    }

    pub(super) fn add(
        &mut self,
        (realm, path): (&str, &Path),
        source: Option<String>,
        classes: &[ExportedClass],
    ) {
        let in_services = path.starts_with(Path::new(realm).join(Self::SERVICE_FOLDER));
        let in_delegates = path.starts_with(Path::new(realm).join(Self::DELEGATE_FOLDER));
        let service = source.as_ref().filter(|_| in_services);
        self.services
            .extend(service.map(|source| (path.to_path_buf(), source.clone())));
        let delegate = Self::own_class(path, classes).filter(|_| in_delegates);
        self.delegates.extend(
            delegate
                .zip(source)
                .map(|(class, source)| Self::delegate(path, class, source)),
        );
    }

    fn own_class<'c>(path: &Path, classes: &'c [ExportedClass]) -> Option<&'c ExportedClass> {
        let stem = path.file_stem().and_then(|stem| stem.to_str());
        classes
            .iter()
            .find(|class| Some(class.name.as_str()) == stem)
    }

    fn delegate(
        path: &Path,
        class: &ExportedClass,
        source: String,
    ) -> (String, Declaration, PathBuf, String) {
        let portable = path.to_string_lossy().replace('\\', "/");
        let declaration = Declaration::new(portable, class.member.clone());
        (class.name.clone(), declaration, path.to_path_buf(), source)
    }

    pub(super) fn blocks(&self) -> String {
        let files = self
            .services
            .iter()
            .map(|(path, source)| (path.as_path(), source.as_bytes()));
        let forwards = DelegateForwards::new(files);
        self.delegates
            .iter()
            .filter_map(|(name, declaration, path, source)| {
                let base = forwards.taken_on(path, source)?;
                let shown = Self::shown(base.service_source, &base.methods);
                (!shown.is_empty()).then(|| declaration.base_lines(name, &base.service, &shown))
            })
            .collect()
    }

    fn shown(service_source: &str, methods: &[(String, usize)]) -> Vec<String> {
        let lines: Vec<&str> = service_source.lines().collect();
        methods
            .iter()
            .filter(|(name, line)| {
                let text = lines.get(*line).map_or("", |text| text.trim_start());
                ![name.as_str(), text].iter().any(|word| {
                    Self::HIDDEN_STARTS
                        .iter()
                        .any(|start| word.starts_with(start))
                })
            })
            .map(|(name, _)| name.clone())
            .collect()
    }
}
