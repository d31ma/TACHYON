use super::java_constructor_provider::JavaConstructorProvider;
use super::java_member::JavaMember;
use super::java_member_kind::JavaMemberKind;
use super::java_value_type::JavaValueType;
use crate::Failure;
use crate::events::{EventListener, EventMethod, EventModule, EventType};
use std::path::Path;

pub(super) struct JavaEventMembers<'a> {
    path: &'a Path,
    class: &'a str,
    values: &'a JavaValueType,
}

impl<'a> JavaEventMembers<'a> {
    const PUBLISHER: &'static str = "ApplicationEventPublisher";
    const PUBLISHER_VALUE: &'static str = " = new ApplicationEventPublisher()";
    const FIELD_MODIFIERS: [&'static str; 4] = ["final ", "public ", "protected ", "transient "];
    const MOST_EVENT_NAMES: usize = 32;
    const MOST_LISTENERS: usize = 128;

    pub(super) fn new(path: &'a Path, class: &'a str, values: &'a JavaValueType) -> Self {
        Self {
            path,
            class,
            values,
        }
    }

    pub(super) fn listeners(
        &self,
        module: Option<&EventModule>,
    ) -> Result<Vec<JavaMember>, Failure> {
        module.map_or_else(|| Ok(Vec::new()), |module| self.module_listeners(module))
    }

    pub(super) fn with_publishers(source: &str, providers: &[JavaConstructorProvider]) -> String {
        source
            .split_inclusive('\n')
            .map(|line| Self::with_publisher(line, providers))
            .collect()
    }

    fn module_listeners(&self, module: &EventModule) -> Result<Vec<JavaMember>, Failure> {
        let listening: Vec<&EventType> = module
            .types
            .iter()
            .filter(|event_type| !event_type.listeners.is_empty())
            .collect();
        listening
            .iter()
            .try_for_each(|event_type| self.check_owner(event_type))?;
        listening
            .iter()
            .try_for_each(|event_type| self.check_bounds(&event_type.listeners))?;
        listening
            .iter()
            .flat_map(|event_type| &event_type.listeners)
            .map(|listener| self.listener(module, listener))
            .collect()
    }

    fn check_owner(&self, event_type: &EventType) -> Result<(), Failure> {
        if event_type.name != self.class {
            return Err(self.failure(&format!(
                "@OnEvent belongs on the service class {}, not on {}.",
                self.class, event_type.name
            )));
        }
        Ok(())
    }

    fn with_publisher(line: &str, providers: &[JavaConstructorProvider]) -> String {
        let declaration = Self::FIELD_MODIFIERS
            .iter()
            .fold(line.trim(), |rest, modifier| {
                rest.strip_prefix(modifier).unwrap_or(rest)
            });
        let is_publisher = declaration
            .strip_prefix(Self::PUBLISHER)
            .and_then(|rest| rest.strip_suffix(';'))
            .filter(|rest| Self::names_one_field(rest))
            .is_some_and(|rest| {
                !providers
                    .iter()
                    .any(|provider| provider.name() == rest.trim())
            });
        match (is_publisher, line.rfind(';')) {
            (true, Some(end)) => {
                format!("{}{}{}", &line[..end], Self::PUBLISHER_VALUE, &line[end..])
            }
            _ => String::from(line),
        }
    }

    fn names_one_field(rest: &str) -> bool {
        let name = rest.trim();
        let is_name = name
            .chars()
            .all(|character| [character.is_alphanumeric(), character == '_'].contains(&true));
        [rest.starts_with(' '), name.is_empty(), is_name] == [true, false, true]
    }

    fn check_bounds(&self, listeners: &[EventListener]) -> Result<(), Failure> {
        let mut names: Vec<&String> = listeners
            .iter()
            .flat_map(|listener| &listener.events)
            .collect();
        names.sort();
        names.dedup();
        let too_many = [
            names.len() > Self::MOST_EVENT_NAMES,
            listeners.len() > Self::MOST_LISTENERS,
        ];
        if too_many.contains(&true) {
            return Err(self.failure(
                "A Java web service listens to 32 events and has 128 listeners at most.",
            ));
        }
        Ok(())
    }

    fn listener(
        &self,
        module: &EventModule,
        listener: &EventListener,
    ) -> Result<JavaMember, Failure> {
        let method = module
            .methods
            .iter()
            .find(|method| {
                (method.name.as_str(), method.owner.as_str())
                    == (listener.method.as_str(), self.class)
            })
            .ok_or_else(|| {
                self.failure(&format!("Cannot find the listener {}.", listener.method))
            })?;
        let parameters = self.parameter(method)?;
        Ok(JavaMember {
            name: method.name.clone(),
            value_type: method.return_type.trim().to_owned(),
            parameters,
            kind: JavaMemberKind::Listener {
                key: format!("__event_{:08}_{}", method.signature.start, method.name),
                description: Self::description(listener),
            },
        })
    }

    fn parameter(&self, method: &EventMethod) -> Result<Vec<String>, Failure> {
        let types: Vec<String> = method
            .parameters
            .iter()
            .map(|parameter| parameter.type_source.trim().to_owned())
            .collect();
        let readable = types
            .iter()
            .all(|kind| self.values.reading(kind, "value").is_some());
        if [types.len() == 1, readable] != [true, true] {
            return Err(self.failure(&format!(
                "The listener {} takes one model from client/models, for example {}(OrderPlaced order).",
                method.name, method.name
            )));
        }
        Ok(types)
    }

    fn description(listener: &EventListener) -> String {
        let described = serde_json::json!({
            "models": listener.models,
            "events": listener.events,
            "replay": listener.replay,
            "order": listener.order,
            "asynchronous": listener.asynchronous,
            "condition": listener.condition,
            "returns": listener.returns,
        });
        described.to_string()
    }

    fn failure(&self, message: &str) -> Failure {
        super::super::host::native_tool_failure(
            1405,
            &format!("{}: {message}", self.path.display()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::JavaEventMembers;

    #[test]
    fn an_uninitialised_publisher_field_is_constructed() {
        let source = "class A {\n    ApplicationEventPublisher applicationEventPublisher;\n      final ApplicationEventPublisher events;\n    ApplicationEventPublisher kept = null;\n    ApplicationEventPublisherX other;\n}\n";
        assert_eq!(
            JavaEventMembers::with_publishers(source, &[]),
            "class A {\n    ApplicationEventPublisher applicationEventPublisher = new ApplicationEventPublisher();\n      final ApplicationEventPublisher events = new ApplicationEventPublisher();\n    ApplicationEventPublisher kept = null;\n    ApplicationEventPublisherX other;\n}\n"
        );
    }
}

#[cfg(test)]
mod publisher_tests {
    use super::{JavaConstructorProvider, JavaEventMembers};

    #[test]
    fn a_constructor_owned_publisher_keeps_its_authored_field_assignment() {
        let source = "class ChoiceService {\n final ApplicationEventPublisher events;\n ApplicationEventPublisher legacy;\n ChoiceService(ApplicationEventPublisher events) { this.events = events; }\n}\n";
        let providers = [JavaConstructorProvider::Publisher(String::from("events"))];
        let compiled = JavaEventMembers::with_publishers(source, &providers);
        assert!(compiled.contains("final ApplicationEventPublisher events;"));
        assert!(compiled.contains("this.events = events;"));
        assert!(
            compiled
                .contains("ApplicationEventPublisher legacy = new ApplicationEventPublisher();")
        );
    }
}
