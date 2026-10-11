use super::{
    Discovery, Module, ModuleInput, invalid, open_parent, read_module_text, test_configuration,
};
use crate::Failure;
use cap_std::fs::Dir;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) struct SelectedTestSetups<'a> {
    root: &'a Dir,
    production: &'a [Module],
    units: Vec<Module>,
    seen: BTreeSet<PathBuf>,
    bytes: usize,
}

impl<'a> SelectedTestSetups<'a> {
    const LANGUAGE: &'static str = "cs";
    const ENVIRONMENT: &'static str = "tac";

    pub(super) fn read(
        root: &'a Dir,
        input: ModuleInput<'_>,
        modules: &'a [Module],
        total: usize,
    ) -> Result<Vec<Module>, Failure> {
        Self::collect(root, input, modules, total)
    }

    fn collect(
        root: &'a Dir,
        input: ModuleInput<'_>,
        modules: &'a [Module],
        total: usize,
    ) -> Result<Vec<Module>, Failure> {
        let mut selected = Self {
            root,
            production: modules,
            units: Vec::new(),
            seen: modules
                .iter()
                .map(|module| module.path.clone())
                .chain(std::iter::once(input.entry.to_path_buf()))
                .collect(),
            bytes: total,
        };
        let client = open_parent(root, Path::new("client/entry"))?;
        let mut discovery = Discovery::new(Self::LANGUAGE);
        discovery.search_role_folder(&client, Path::new("client/tests"), "tests")?;
        discovery
            .paths
            .iter()
            .try_for_each(|path| selected.unit(path))?;
        Ok(selected.units)
    }

    fn unit(&mut self, path: &Path) -> Result<(), Failure> {
        let source = read_module_text(
            self.root,
            path,
            Module::MAX_SOURCE.saturating_sub(self.bytes),
        )?;
        self.account(path, &source)?;
        crate::behavior::test_configuration_name(path, &source, Self::ENVIRONMENT)?
            .map(|_| self.selected(path, &source))
            .transpose()
            .map(|_| ())
    }

    fn selected(&mut self, path: &Path, source: &str) -> Result<(), Failure> {
        crate::annotation_layout::validate(path, source)?;
        let mut modules: Vec<Module> = self
            .production
            .iter()
            .map(|module| Module {
                path: module.path.clone(),
                source: module.source.clone(),
            })
            .collect();
        test_configuration::include(
            self.root,
            path,
            source,
            Self::LANGUAGE,
            Self::ENVIRONMENT,
            &mut modules,
            self.bytes,
        )?;
        modules
            .into_iter()
            .skip(self.production.len())
            .try_for_each(|module| self.setup(module))
    }

    fn setup(&mut self, module: Module) -> Result<(), Failure> {
        self.account(&module.path, &module.source)?;
        self.append(module);
        Ok(())
    }

    fn append(&mut self, module: Module) {
        if !self.units.iter().any(|unit| unit.path == module.path) {
            self.units.push(module);
        }
    }

    fn account(&mut self, path: &Path, source: &str) -> Result<(), Failure> {
        self.record(path, source);
        self.bound(path)
    }

    fn record(&mut self, path: &Path, source: &str) {
        if self.seen.insert(path.to_path_buf()) {
            self.bytes += source.len();
        }
    }

    fn bound(&self, path: &Path) -> Result<(), Failure> {
        if self.seen.len() > Module::MAX_MODULES || self.bytes > Module::MAX_SOURCE {
            return Err(invalid(
                path,
                "Production and selected test ownership material may compose at most 512 files and 1 MiB.",
            ));
        }
        Ok(())
    }
}
