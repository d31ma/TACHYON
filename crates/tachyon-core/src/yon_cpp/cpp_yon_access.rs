use crate::tac_cpp::CppAttribute;
use crate::yon::route_access::RouteAccess;

pub(crate) struct CppYonAccess;

impl CppYonAccess {
    pub(crate) fn read(markers: &[CppAttribute]) -> RouteAccess {
        let found = markers.iter().find_map(Self::one);
        found.unwrap_or_default()
    }

    fn one(marker: &CppAttribute) -> Option<RouteAccess> {
        match marker.name.as_str() {
            "Public" => Some(RouteAccess::Public),
            "SignedIn" => Some(RouteAccess::SignedIn),
            "Allow" => Some(RouteAccess::Allow(Self::roles(&marker.arguments))),
            _ => None,
        }
    }

    fn roles(arguments: &str) -> Vec<String> {
        arguments
            .split(',')
            .map(str::trim)
            .filter(|role| !role.is_empty())
            .map(str::to_owned)
            .collect()
    }
}
