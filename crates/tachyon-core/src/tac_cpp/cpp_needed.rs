use super::CppService;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct CppNeeded;

impl CppNeeded {
    pub(crate) fn attach(services: &mut [CppService], registered: &[(String, String)]) {
        let instances = Self::instances(registered);
        let needs = Self::direct_needs(services);
        Self::fill(services, &needs, &instances);
    }

    fn fill(
        services: &mut [CppService],
        needs: &BTreeMap<String, Vec<String>>,
        instances: &BTreeMap<String, String>,
    ) {
        for service in services.iter_mut() {
            service.needed = Self::reached(&service.class().name, needs, instances);
        }
    }

    fn instances(registered: &[(String, String)]) -> BTreeMap<String, String> {
        registered
            .iter()
            .map(|(instance, class)| (class.clone(), instance.clone()))
            .collect()
    }

    fn direct_needs(services: &[CppService]) -> BTreeMap<String, Vec<String>> {
        let shared = Self::shared_classes(services);
        services
            .iter()
            .map(|service| {
                (
                    service.class().name.clone(),
                    Self::shared_needs(service, &shared),
                )
            })
            .collect()
    }

    fn shared_classes(services: &[CppService]) -> BTreeSet<String> {
        services
            .iter()
            .filter(|service| !service.is_prototype())
            .map(|service| service.class().name.clone())
            .collect()
    }

    fn shared_needs(service: &CppService, shared: &BTreeSet<String>) -> Vec<String> {
        service
            .needs()
            .into_iter()
            .map(|(_, provider)| provider)
            .filter(|provider| shared.contains(provider))
            .collect()
    }

    fn reached(
        root: &str,
        needs: &BTreeMap<String, Vec<String>>,
        instances: &BTreeMap<String, String>,
    ) -> Vec<String> {
        let mut seen = vec![root.to_owned()];
        let mut pending = vec![root.to_owned()];
        while let Some(class) = pending.pop() {
            Self::visit(needs.get(&class), &mut seen, &mut pending);
        }
        seen.iter()
            .skip(1)
            .filter_map(|class| instances.get(class).cloned())
            .collect()
    }

    fn visit(needed: Option<&Vec<String>>, seen: &mut Vec<String>, pending: &mut Vec<String>) {
        for class in needed.into_iter().flatten() {
            Self::add(class, seen, pending);
        }
    }

    fn add(class: &str, seen: &mut Vec<String>, pending: &mut Vec<String>) {
        if seen.iter().any(|known| known == class) {
            return;
        }
        seen.push(class.to_owned());
        pending.push(class.to_owned());
    }
}
