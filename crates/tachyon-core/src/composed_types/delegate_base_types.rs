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
        let Some(source) = source else {
            return;
        };
        if path.starts_with(Path::new(realm).join(Self::SERVICE_FOLDER)) {
            self.services.push((path.to_path_buf(), source));
            return;
        }
        let stem = path.file_stem().and_then(|stem| stem.to_str());
        let delegate = classes
            .iter()
            .find(|class| Some(class.name.as_str()) == stem);
        let in_delegates = path.starts_with(Path::new(realm).join(Self::DELEGATE_FOLDER));
        if let Some(class) = delegate.filter(|_| in_delegates) {
            let portable = path.to_string_lossy().replace('\\', "/");
            let declaration = Declaration::new(portable, class.member.clone());
            self.delegates
                .push((class.name.clone(), declaration, path.to_path_buf(), source));
        }
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
