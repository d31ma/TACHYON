use crate::unicode_name::UnicodeName;

pub(super) struct RustCollaborators {
    names: Vec<String>,
}

impl RustCollaborators {
    const DECLARATIONS: [&'static str; 3] = ["pub(crate) struct ", "pub struct ", "struct "];

    pub(super) fn declared_beside(service: &str, source: &str) -> Self {
        let names = source
            .lines()
            .filter_map(Self::declared_name)
            .filter(|name| name != service)
            .collect();
        Self { names }
    }

    pub(super) fn holds(&self, kind: Option<&str>) -> bool {
        kind.is_some_and(|kind| self.names.iter().any(|name| name == kind.trim()))
    }

    fn declared_name(line: &str) -> Option<String> {
        let line = line.trim_start();
        let rest = Self::DECLARATIONS
            .iter()
            .find_map(|keyword| line.strip_prefix(keyword))?;
        let name = UnicodeName::leading(rest);
        (!name.is_empty()).then_some(name)
    }
}

#[cfg(test)]
mod tests {
    use super::RustCollaborators;

    #[test]
    fn every_struct_beside_the_service_is_a_collaborator() {
        let source = "struct CatalogService { catalog: CatalogClient }\n\
                      struct CatalogClient { behaviors: BehaviorContext }\n\
                      pub struct CatalogSetup {}\n    struct CatalogTransport;\n";
        let collaborators = RustCollaborators::declared_beside("CatalogService", source);
        for kind in ["CatalogClient", "CatalogSetup", "CatalogTransport"] {
            assert!(collaborators.holds(Some(kind)), "{kind}");
        }
        for kind in [Some("CatalogService"), Some("i64"), None] {
            assert!(!collaborators.holds(kind), "{kind:?}");
        }
    }
}
