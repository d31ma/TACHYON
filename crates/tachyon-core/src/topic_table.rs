use crate::topics::Topics;
use crate::yon::route_access::RouteAccess;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TopicTable {
    rules: BTreeMap<String, RouteAccess>,
}

impl TopicTable {
    pub(crate) fn insert(&mut self, name: &str, access: RouteAccess) {
        self.rules
            .extend(Topics::slug(name).map(|slug| (slug, access)));
    }

    pub(crate) fn access(&self, slug: &str) -> Option<&RouteAccess> {
        self.rules.get(slug)
    }
}
