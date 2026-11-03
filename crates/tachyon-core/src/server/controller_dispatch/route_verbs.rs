use crate::routing::RouteMethod;
use crate::yon::plan::Route;
use tachyon_contracts::HttpMethod;

pub(in crate::server) struct RouteVerbs;

impl RouteVerbs {
    pub(in crate::server) fn allowed(routes: &[&Route]) -> Vec<HttpMethod> {
        let mut verbs: Vec<HttpMethod> = routes
            .iter()
            .filter_map(|route| RouteMethod::parse(&route.verb))
            .flat_map(Self::with_head)
            .collect();
        verbs.push(HttpMethod::Options);
        verbs.sort_by_key(|verb| RouteMethod::name(*verb));
        verbs.dedup();
        verbs
    }

    pub(in crate::server) fn wanted(method: HttpMethod) -> &'static str {
        RouteMethod::name(Self::answered_as(method))
    }

    fn with_head(verb: HttpMethod) -> Vec<HttpMethod> {
        if verb == HttpMethod::Get {
            return vec![HttpMethod::Get, HttpMethod::Head];
        }
        vec![verb]
    }

    fn answered_as(method: HttpMethod) -> HttpMethod {
        if method == HttpMethod::Head {
            return HttpMethod::Get;
        }
        method
    }
}
