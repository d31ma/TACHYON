use super::selected_test_setups::SelectedTestSetups;
use super::{HelperCallers, Module, ModuleInput, invalid};
use crate::Failure;
use crate::behavior::CsharpHelperGraph;
use cap_std::fs::Dir;
use std::collections::BTreeSet;

pub(super) struct CsharpTestHelpers;

impl CsharpTestHelpers {
    const ENVIRONMENT: &'static str = "tac";
    const HELPER_FOLDER: &'static str = "client/helpers";

    pub(super) fn prune(
        root: &Dir,
        input: ModuleInput<'_>,
        modules: &mut Vec<Module>,
        total: usize,
    ) -> Result<(), Failure> {
        Self::wanted(input)
            .then(|| Self::production(root, input, modules, total))
            .transpose()
            .map(|_| ())
    }

    fn wanted(input: ModuleInput<'_>) -> bool {
        matches!(
            (
                input.entry.extension().and_then(|value| value.to_str()),
                input.environment,
                input.entry.starts_with("client/tests")
            ),
            (Some("cs" | "fs"), Self::ENVIRONMENT, false)
        )
    }

    fn production(
        root: &Dir,
        input: ModuleInput<'_>,
        modules: &mut Vec<Module>,
        total: usize,
    ) -> Result<(), Failure> {
        Self::has_helpers(modules)
            .then(|| Self::validated(root, input, modules, total))
            .transpose()
            .map(|_| ())
    }

    fn has_helpers(modules: &[Module]) -> bool {
        modules
            .iter()
            .any(|module| module.path.starts_with(Self::HELPER_FOLDER))
    }

    fn validated(
        root: &Dir,
        input: ModuleInput<'_>,
        modules: &mut Vec<Module>,
        total: usize,
    ) -> Result<(), Failure> {
        modules
            .iter()
            .try_for_each(|module| Self::validate_helper(root, module))?;
        Self::completed(root, input, modules, total)
    }

    fn completed(
        root: &Dir,
        input: ModuleInput<'_>,
        modules: &mut Vec<Module>,
        total: usize,
    ) -> Result<(), Failure> {
        let graph = HelperCallers::graph(input.entry, input.source, modules)?;
        let production = Self::production_names(input, modules, graph.helpers())?;
        let owned = graph.owned(&production);
        if graph.helpers().is_subset(&owned) {
            return HelperCallers::check(input.entry, input.source, modules, Self::ENVIRONMENT);
        }
        Self::with_tests(root, input, modules, total)
    }

    fn with_tests(
        root: &Dir,
        input: ModuleInput<'_>,
        modules: &mut Vec<Module>,
        total: usize,
    ) -> Result<(), Failure> {
        let selected = SelectedTestSetups::read(root, input, modules, total)?;
        let mut units: Vec<Module> = modules
            .iter()
            .map(|module| Module {
                path: module.path.clone(),
                source: module.source.clone(),
            })
            .collect();
        units.extend(selected);
        HelperCallers::check(input.entry, input.source, &units, Self::ENVIRONMENT)?;
        let graph = HelperCallers::graph(input.entry, input.source, &units)?;
        let production = Self::production_names(input, modules, graph.helpers())?;
        let all_roots = Self::production_names(input, &units, graph.helpers())?;
        let owned = graph.owned(&all_roots);
        modules
            .iter()
            .try_for_each(|module| Self::require_root(module, &graph, &owned))?;
        let retained = graph.referenced(&production);
        modules.retain(|module| Self::keep(module, graph.helpers(), &retained));
        Ok(())
    }

    fn validate_helper(root: &Dir, module: &Module) -> Result<(), Failure> {
        if module.path.starts_with(Self::HELPER_FOLDER) {
            let raw = super::read_module_text(root, &module.path, Module::MAX_SOURCE)?;
            crate::annotation_layout::validate(&module.path, &raw)?;
        }
        Ok(())
    }

    fn production_names(
        input: ModuleInput<'_>,
        modules: &[Module],
        helpers: &BTreeSet<String>,
    ) -> Result<BTreeSet<String>, Failure> {
        let mut names: BTreeSet<String> =
            crate::annotation_layout::declaration_names(input.entry, input.source)?
                .into_iter()
                .collect();
        modules
            .iter()
            .try_for_each(|module| Self::names(module, &mut names))?;
        names.retain(|name| !helpers.contains(name));
        Ok(names)
    }

    fn names(module: &Module, names: &mut BTreeSet<String>) -> Result<(), Failure> {
        names.extend(crate::annotation_layout::declaration_names(
            &module.path,
            &module.source,
        )?);
        Ok(())
    }

    fn require_root(
        module: &Module,
        graph: &CsharpHelperGraph,
        owned: &BTreeSet<String>,
    ) -> Result<(), Failure> {
        let names = Self::declared_helpers(module, graph.helpers())?;
        Self::require_owned(module, names.is_subset(owned))
    }

    fn require_owned(module: &Module, owned: bool) -> Result<(), Failure> {
        if !owned {
            return Err(invalid(
                &module.path,
                "A caller-owned Helper needs a production or selected TestSetup constructor root; a Helper constructor cycle is not ownership.",
            ));
        }
        Ok(())
    }

    fn keep(module: &Module, helpers: &BTreeSet<String>, retained: &BTreeSet<String>) -> bool {
        Self::declared_helpers(module, helpers).is_ok_and(|names| names.is_subset(retained))
    }

    fn declared_helpers(
        module: &Module,
        helpers: &BTreeSet<String>,
    ) -> Result<BTreeSet<String>, Failure> {
        let names = crate::annotation_layout::declaration_names(&module.path, &module.source)?;
        let declared: BTreeSet<String> = names.into_iter().collect();
        Ok(declared.intersection(helpers).cloned().collect())
    }
}
