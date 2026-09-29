use super::contract_member::ContractMember;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContractRole {
    pub(crate) marker: &'static str,
    pub(crate) titled: &'static str,
    pub(crate) normalized: &'static str,
    pub(crate) suffix: &'static str,
    pub(crate) noun: &'static str,
    pub(crate) works: &'static str,
    pub(crate) no_implementation: &'static str,
    pub(crate) help: &'static str,
    pub(crate) names_entity: bool,
    pub(crate) managed_in_browser: bool,
    member_markers: &'static [&'static str],
    member_rule: &'static str,
}

impl ContractRole {
    const CLIENT_CALLS: [&'static str; 8] = [
        "getrequest",
        "postrequest",
        "putrequest",
        "patchrequest",
        "deleterequest",
        "streamrequest",
        "socketrequest",
        "gateway",
    ];
    const STORE_CALLS: [&'static str; 1] = ["query"];
    const CLIENT: Self = Self {
        marker: "HTTPClient",
        titled: "An HTTPClient",
        normalized: "httpclient",
        suffix: "Client",
        noun: "client",
        works: "requests",
        no_implementation: "An HTTPClient has no implementation: Tachyon generates each request. Move the logic into a service.",
        help: "An HTTPClient is a contract, like a Spring @HttpExchange interface: each method is a signature that says which request it sends, and Tachyon writes the body, the constructor and the behavior context. `ty migrate fix --write` rewrites a class client. See docs/BEHAVIOR_ANNOTATIONS.md.",
        names_entity: false,
        managed_in_browser: true,
        member_markers: &Self::CLIENT_CALLS,
        member_rule: "{owner}.{method} has no request annotation. Each method of an HTTPClient sends a request: mark it with {markers}",
    };
    const STORE: Self = Self {
        marker: "Store",
        titled: "A Store",
        normalized: "store",
        suffix: "Store",
        noun: "store",
        works: "queries",
        no_implementation: "A Store has no implementation: Tachyon generates each query. Move the logic into a service.",
        help: "A Store is a contract, like a Spring Data repository interface: it names its Entity, each method is a derived query (findBy, countBy or existsBy) or a Query, and Tachyon writes the body, the constructor and the behavior context from the Setup. A service or controller injects it with Needs. AllOrNothing goes on the service method: a store call inside it joins the open transaction, like Spring's @Transactional. `ty migrate fix --write` rewrites a class store. See docs/BEHAVIOR_ANNOTATIONS.md.",
        names_entity: true,
        managed_in_browser: false,
        member_markers: &Self::STORE_CALLS,
        member_rule: "{owner}.{method} is not a query. Each method of a Store runs a query: start its name with findBy, countBy or existsBy (find_by_, count_by_ or exists_by_ in snake_case), or mark it with {markers}",
    };
    const ALL: [Self; 2] = [Self::CLIENT, Self::STORE];
    const OTHER_ROLES: [&'static str; 12] = [
        "httpclient",
        "service",
        "store",
        "helper",
        "converter",
        "setup",
        "entity",
        "controller",
        "requestroute",
        "socketroute",
        "identityprovider",
        "errorhandler",
    ];

    pub(crate) fn of_marker(normalized: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|role| role.normalized == normalized)
    }

    pub(crate) fn is_mentioned(source: &str) -> bool {
        let folded = source.to_ascii_lowercase().replace('_', "");
        Self::ALL
            .iter()
            .any(|role| folded.contains(role.normalized))
    }

    pub(crate) fn is_other_role(self, normalized: &str) -> bool {
        [
            Self::OTHER_ROLES.contains(&normalized),
            normalized == self.normalized,
        ] == [true, false]
    }

    pub(crate) fn member_markers(self) -> &'static [&'static str] {
        self.member_markers
    }

    pub(crate) fn is_call(self, member: &ContractMember, code: &str) -> bool {
        let derived = [
            self.names_entity,
            crate::behavior::is_derived_query(&crate::go_source::lower_first(&member.name(code))),
        ] == [true, true];
        [member.is_marked_with(self.member_markers), derived].contains(&true)
    }

    pub(crate) fn member_rule(self, owner: &str, method: &str, markers: &str) -> String {
        self.member_rule
            .replace("{owner}", owner)
            .replace("{method}", method)
            .replace("{markers}", markers)
    }

    pub(crate) fn needs_context(self, members: &[ContractMember]) -> bool {
        let requests = members.iter().any(ContractMember::sends_request);
        [self.names_entity, requests].contains(&true)
    }
}
