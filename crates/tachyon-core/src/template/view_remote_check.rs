use super::{ServiceReach, ServiceRegistry};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeSet;

pub(crate) struct ViewRemoteCheck;

impl ViewRemoteCheck {
    const CODE: u16 = 1377;
    const REMOTE_SUFFIXES: [&'static str; 3] = ["Client", "Remote", "Api"];
    const SERVICE_SUFFIX: &'static str = "Service";

    pub(crate) fn run(
        services: &ServiceRegistry,
        view_names: &BTreeSet<String>,
        source_path: &str,
    ) -> Result<(), Failure> {
        let named = view_names
            .iter()
            .find_map(|name| Self::named_remote(services, name));
        named.map_or(Ok(()), |(name, class)| {
            Err(Self::failure(services, name, class, source_path))
        })
    }

    fn named_remote<'a>(
        services: &'a ServiceRegistry,
        name: &'a str,
    ) -> Option<(&'a str, &'a str)> {
        let service = services.get(name);
        let remote = services.remote_class(name).map(|class| (name, class));
        remote.filter(|_| service.is_none())
    }

    fn failure(services: &ServiceRegistry, name: &str, class: &str, source_path: &str) -> Failure {
        let message = format!(
            "The view '{source_path}' names the remote `{name}` ({class} in client/remotes). A view reaches only client services, never a remote."
        );
        let help = Self::help(services, name, class);
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(help),
            source_span(source_path, 0, 0),
        ))
    }

    fn help(services: &ServiceRegistry, name: &str, class: &str) -> String {
        let wrappers = Self::wrappers(services, name, class);
        let service = wrappers
            .first()
            .cloned()
            .unwrap_or_else(|| Self::new_service_name(class));
        format!(
            "Call a service from the view, and let the service call the remote. {service} could wrap {class}: inject it with `@Needs`, add a method that calls `{name}`, and call that method from the view (ADR 0087, TY1327)."
        )
    }

    fn wrappers(services: &ServiceRegistry, name: &str, class: &str) -> Vec<String> {
        let remote_names = BTreeSet::from([name.to_owned(), class.to_owned()]);
        services
            .iter()
            .filter(|(_, service)| !ServiceReach::needs(service, &remote_names).is_empty())
            .map(|(_, service)| service.class().to_owned())
            .collect()
    }

    fn new_service_name(class: &str) -> String {
        let base = Self::REMOTE_SUFFIXES
            .iter()
            .find_map(|suffix| class.strip_suffix(suffix))
            .filter(|base| !base.is_empty())
            .unwrap_or(class);
        format!("a new {base}{}", Self::SERVICE_SUFFIX)
    }
}
