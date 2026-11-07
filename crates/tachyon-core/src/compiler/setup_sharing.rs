use super::module_code::ModuleCode;
use super::setup_class::SetupClass;
use super::setup_registrations::SetupRegistrations;
use std::collections::BTreeMap;

pub(super) struct SetupSharing {
    classes: BTreeMap<String, String>,
}

impl SetupSharing {
    const MINIMUM_MODULES: usize = 2;

    pub(super) fn plan(modules: &[&[u8]]) -> Self {
        let mut found = BTreeMap::new();
        modules
            .iter()
            .map(|bytes| ModuleCode::read(bytes))
            .for_each(|module| Self::record(&mut found, &module));
        let classes = found
            .into_iter()
            .filter_map(|(name, texts)| Self::one_text(&texts).map(|text| (name, text)))
            .collect();
        Self { classes }
    }

    pub(super) fn without_setups(&self, bytes: &[u8]) -> (Vec<u8>, Vec<String>) {
        let module = ModuleCode::read(bytes);
        let mut classes: Vec<SetupClass> = self
            .classes
            .iter()
            .filter_map(|(name, text)| Self::same_class(&module, name, text))
            .collect();
        classes.sort_by_key(|class| std::cmp::Reverse(class.span.start));
        let names = classes
            .iter()
            .rev()
            .map(|class| class.name.clone())
            .collect();
        (Self::removed(module.text, &classes).into_bytes(), names)
    }

    pub(super) fn exports(&self) -> Vec<u8> {
        self.classes
            .values()
            .flat_map(|text| format!("\nexport {text}\n").into_bytes())
            .collect()
    }

    fn record(found: &mut BTreeMap<String, Vec<Option<String>>>, module: &ModuleCode) {
        for name in SetupRegistrations::names(module) {
            let class = SetupClass::shareable(module, &name);
            let text = class.map(|class| class.text(module).to_owned());
            found.entry(name).or_default().push(text);
        }
    }

    fn one_text(texts: &[Option<String>]) -> Option<String> {
        let first = texts.first()?.clone()?;
        let same = texts
            .iter()
            .all(|text| text.as_deref() == Some(first.as_str()));
        let enough = texts.len() >= Self::MINIMUM_MODULES;
        ([same, enough] == [true, true]).then_some(first)
    }

    fn same_class(module: &ModuleCode, name: &str, text: &str) -> Option<SetupClass> {
        SetupClass::find(module, name).filter(|class| class.text(module) == text)
    }

    fn removed(mut text: String, classes: &[SetupClass]) -> String {
        for class in classes {
            text.replace_range(class.span.clone(), "");
        }
        text
    }
}
