use crate::template::{ServiceReach, ServiceRegistry};
use std::collections::BTreeSet;

pub(super) struct NativeServiceRoots;

impl NativeServiceRoots {
    pub(super) fn expand(registry: &ServiceRegistry, roots: &BTreeSet<String>) -> BTreeSet<String> {
        ServiceReach::expand(registry, roots)
    }
}
