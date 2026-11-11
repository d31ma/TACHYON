use super::{registry::NativeCompanionInput, rust_service_plan::RustServicePlan};
use crate::Failure;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

#[derive(Default)]
pub(super) struct RustHeldServices {
    dependencies: BTreeMap<String, BTreeSet<String>>,
}

impl RustHeldServices {
    pub(super) fn record(&mut self, name: &str, dependencies: &BTreeSet<String>) {
        self.dependencies
            .insert(name.to_owned(), dependencies.clone());
    }

    pub(super) fn tables(
        &self,
        source: &str,
        services: &BTreeMap<String, RustServicePlan>,
    ) -> Result<Vec<(String, String)>, Failure> {
        self.saved(services)
            .into_iter()
            .map(|service| Self::table(source, service))
            .collect()
    }

    pub(super) fn dispatch(
        &self,
        inputs: &[&NativeCompanionInput],
        services: &BTreeMap<String, RustServicePlan>,
    ) -> String {
        let needs = self.needs_entries(inputs, services);
        let held = self.held_entries(services);
        format!(
            "fn tac_service_needs(service: &str) -> &'static [&'static str] {{ match service {{ {needs}_ => &[] }} }}\nfn tac_service_held(held: &str) -> Option<Vec<(&'static str, TacMember)>> {{ let activation = NativePageServices::activation()?; let result: Option<Result<Vec<(&'static str, TacMember)>, String>> = match held {{ {held}_ => None }}; result?.map_err(|failure| activation.fail(failure)).ok() }}\n"
        )
    }

    fn table(source: &str, service: &RustServicePlan) -> Result<(String, String), Failure> {
        let path = service.path.to_string_lossy();
        let table = crate::companion::RustGraph::held_table(source, &path, &service.saved)?;
        Ok((service.name.clone(), table))
    }

    fn needs_entries(
        &self,
        inputs: &[&NativeCompanionInput],
        services: &BTreeMap<String, RustServicePlan>,
    ) -> String {
        let mut entries = String::new();
        for input in inputs.iter().filter(|input| !input.application_provider) {
            let names = self.needs(&input.class, services);
            let _ = write!(entries, "{:?} => &[{names}], ", input.service);
        }
        entries
    }

    fn held_entries(&self, services: &BTreeMap<String, RustServicePlan>) -> String {
        let mut entries = String::new();
        for service in self.saved(services) {
            let _ = write!(
                entries,
                "{:?} => Some(TachyonRustServices::tac_held_{}(&activation.scope)), ",
                crate::template::instance_name(&service.name),
                service.name
            );
        }
        entries
    }

    fn needs(&self, root: &str, services: &BTreeMap<String, RustServicePlan>) -> String {
        self.reached(root)
            .iter()
            .filter_map(|name| services.get(name))
            .filter(|service| !service.prototype)
            .map(|service| format!("{:?}", crate::template::instance_name(&service.name)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn saved<'a>(
        &self,
        services: &'a BTreeMap<String, RustServicePlan>,
    ) -> Vec<&'a RustServicePlan> {
        self.held()
            .iter()
            .filter_map(|name| services.get(name))
            .filter(|service| Self::keeps_saved_fields(service))
            .collect()
    }

    fn keeps_saved_fields(service: &RustServicePlan) -> bool {
        [service.prototype, service.saved.is_empty()] == [false; 2]
    }

    fn held(&self) -> BTreeSet<String> {
        self.dependencies.values().flatten().cloned().collect()
    }

    fn reached(&self, root: &str) -> BTreeSet<String> {
        let mut reached = self.direct(root);
        let mut size = 0;
        while reached.len() != size {
            size = reached.len();
            reached = self.expand(&reached);
        }
        reached
    }

    fn expand(&self, names: &BTreeSet<String>) -> BTreeSet<String> {
        names
            .iter()
            .flat_map(|name| self.direct(name))
            .chain(names.iter().cloned())
            .collect()
    }

    fn direct(&self, name: &str) -> BTreeSet<String> {
        self.dependencies.get(name).cloned().unwrap_or_default()
    }
}
