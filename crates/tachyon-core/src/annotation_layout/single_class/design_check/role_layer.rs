#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RoleLayer {
    EntryPoint,
    Service,
    Helper,
    DataAccess,
    Other,
    Utility,
    Plain,
}

impl RoleLayer {
    const FOLDERS: [(&'static str, Self, &'static str); 14] = [
        ("controllers", Self::EntryPoint, "a controller"),
        ("services", Self::Service, "a service"),
        ("helpers", Self::Helper, "a helper"),
        ("stores", Self::DataAccess, "a store"),
        ("repositories", Self::DataAccess, "a repository"),
        ("remotes", Self::DataAccess, "a client"),
        ("clients", Self::DataAccess, "a client"),
        ("converters", Self::Other, "a converter"),
        ("mappers", Self::Other, "a mapper"),
        ("configurations", Self::Other, "a configuration"),
        ("setups", Self::Other, "a setup"),
        ("workers", Self::Other, "a worker"),
        ("features", Self::Other, "a feature"),
        ("utils", Self::Utility, "a utility"),
    ];
    const PLAIN_LABEL: &'static str = "a class";
    const RANKED: [Self; 4] = [
        Self::EntryPoint,
        Self::Service,
        Self::Helper,
        Self::DataAccess,
    ];

    pub(super) fn of(folder: &str) -> Self {
        Self::entry(folder).map_or(Self::Plain, |entry| entry.1)
    }

    pub(super) fn label(folder: &str) -> &'static str {
        Self::entry(folder).map_or(Self::PLAIN_LABEL, |entry| entry.2)
    }

    pub(super) fn rank(self) -> Option<usize> {
        Self::RANKED.iter().position(|layer| *layer == self)
    }

    pub(super) fn is_injected(self) -> bool {
        ![Self::Plain, Self::Utility].contains(&self)
    }

    pub(super) fn is_utility(self) -> bool {
        self == Self::Utility
    }

    fn entry(folder: &str) -> Option<&'static (&'static str, Self, &'static str)> {
        Self::FOLDERS.iter().find(|entry| entry.0 == folder)
    }
}
