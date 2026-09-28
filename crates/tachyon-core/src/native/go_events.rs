use crate::Failure;
use crate::companion::go::{GoResults, masked_code, parameter_types};
use crate::events::{EventListener, EventMethod, EventModule, EventType};
use crate::go_source::{self, GoMethod};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

#[derive(Default)]
pub(super) struct GoEvents {
    pub(super) excluded: Vec<String>,
    pub(super) entries: String,
}

struct EventTarget<'a> {
    event: &'a EventMethod,
    declared: &'a GoMethod,
    code: &'a str,
}

fn failure(path: &Path, message: &str) -> Failure {
    super::host::native_tool_failure(1405, &format!("{}: {message}", path.display()))
}

pub(super) fn entries(
    module: Option<&EventModule>,
    class: &str,
    path: &Path,
) -> Result<GoEvents, Failure> {
    let mut events = GoEvents::default();
    let Some(module) = module else {
        return Ok(events);
    };
    let Some(metadata) = owned_metadata(module, class, path)? else {
        return Ok(events);
    };
    let code = masked_code(&module.source, &path.to_string_lossy())?;
    let declared = go_source::methods(&code);
    let mut seen = BTreeSet::new();
    for event in &module.methods {
        if !seen.insert(event.name.as_str()) {
            continue;
        }
        let Some(found) = declared
            .iter()
            .find(|method| method.receiver_type == class && method.name == event.name)
        else {
            return Err(failure(
                path,
                "Go event and lifecycle annotations belong to a method of the service struct.",
            ));
        };
        let target = EventTarget {
            event,
            declared: found,
            code: &code,
        };
        events.excluded.push(event.name.clone());
        events.entries.push_str(&entry(&target, metadata, path)?);
    }
    Ok(events)
}

fn owned_metadata<'a>(
    module: &'a EventModule,
    class: &str,
    path: &Path,
) -> Result<Option<&'a EventType>, Failure> {
    if module.types.len() > 1 {
        return Err(failure(
            path,
            "Native events belong to the page companion type only.",
        ));
    }
    let Some(metadata) = module.types.first() else {
        return Ok(None);
    };
    if metadata.name != class {
        return Err(failure(
            path,
            "Native event annotations must belong to the page companion, not an unowned helper type.",
        ));
    }
    let mut topics = BTreeSet::new();
    for listener in &metadata.listeners {
        for event in &listener.events {
            topics.insert(event.as_str());
        }
    }
    if topics.len() > 32 || metadata.listeners.len() > 128 {
        return Err(failure(
            path,
            "Native event metadata exceeds its 32-topic/128-listener bounds.",
        ));
    }
    Ok(Some(metadata))
}

fn entry(target: &EventTarget<'_>, metadata: &EventType, path: &Path) -> Result<String, Failure> {
    let Some(results) = GoResults::read(&target.code[target.declared.results.clone()]) else {
        return Err(failure(
            path,
            "A Go event method returns nothing, one value, an error, or a value and an error.",
        ));
    };
    let Some(parameters) = parameter_types(target.code, target.declared) else {
        return Err(failure(
            path,
            "Native listener parameters require portable JSON types.",
        ));
    };
    let key = format!(
        "__event_{:08}_{}",
        target.event.signature.start, target.event.name
    );
    let listener = metadata
        .listeners
        .iter()
        .find(|listener| listener.method == target.event.name);
    if let Some(listener) = listener {
        return Ok(listener_entry(target, listener, &key, &parameters, results));
    }
    let destroy = metadata.pre_destroy.as_ref() == Some(&target.event.name);
    let call = format!("instance.{}()", target.event.name);
    Ok(format!(
        "\t\t{{Name: {key:?}, Destroy: {destroy}, Hook: func() error {{\n\t\t\t{}\n\t\t}}}},\n",
        results.hook(&call)
    ))
}

fn listener_entry(
    target: &EventTarget<'_>,
    listener: &EventListener,
    key: &str,
    parameters: &[String],
    results: GoResults,
) -> String {
    let events = quoted_list(&listener.events);
    let condition = listener
        .condition
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let models = listener
        .models
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let returns = listener.returns.clone().unwrap_or_default();
    let mut output = format!(
        "\t\t{{Name: {key:?}, Listener: &tac.Listener{{Events: []string{{{events}}}, Replay: {}, Order: {}, Condition: {}, Models: {}, Returns: {}, Asynchronous: {}, Invoke: func(value any) (any, error) {{\n",
        listener.replay,
        listener.order,
        go_string(&condition),
        go_string(&models),
        go_string(&returns),
        listener.asynchronous
    );
    let mut passed = Vec::new();
    for (index, type_source) in parameters.iter().enumerate() {
        let _ = write!(
            output,
            "\t\t\tvar argument{index} {type_source}\n\t\t\tif err := tac.Assign(&argument{index}, value); err != nil {{\n\t\t\t\treturn nil, err\n\t\t\t}}\n"
        );
        passed.push(format!("argument{index}"));
    }
    let call = format!("instance.{}({})", target.event.name, passed.join(", "));
    let _ = write!(output, "\t\t\t{}\n\t\t}}}}}},\n", results.returned(&call));
    output
}

fn quoted_list(values: &[String]) -> String {
    let mut quoted = Vec::new();
    for value in values {
        quoted.push(go_string(value));
    }
    quoted.join(", ")
}

fn go_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| String::from("\"\""))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    struct Fixtures;

    impl Fixtures {
        const PATH: &str = "client/services/ProbeService.go";
        const SERVICE: &str = "type ProbeService struct {\n\tCount int\n}\n\n// @OnStart()\nfunc (probe *ProbeService) Start() {\n}\n\n// @OnStop()\nfunc (probe *ProbeService) Stop() error {\n\treturn nil\n}\n\n// @OnEvent({\"condition\":\"value.count > 1\"})\n// @InBackground\nfunc (probe *ProbeService) Receive(value Probe) (Probe, error) {\n\treturn value, nil\n}\n";
    }

    fn prepared(class: &str) -> Result<GoEvents, Failure> {
        let path = Path::new(Fixtures::PATH);
        let module = crate::events::extract(path, Fixtures::SERVICE, "tac").expect("events");
        entries(module.as_ref(), class, path)
    }

    #[test]
    fn lifecycle_and_listener_methods_become_hidden_entries() {
        let events = prepared("ProbeService").expect("entries");
        assert_eq!(events.excluded, ["Start", "Stop", "Receive"]);
        let entries = &events.entries;
        assert!(
            entries.contains(
                "Destroy: false, Hook: func() error {\n\t\t\tinstance.Start()\n\t\t\treturn nil\n"
            ),
            "{entries}"
        );
        assert!(
            entries.contains("Destroy: true, Hook: func() error {\n\t\t\treturn instance.Stop()\n"),
            "{entries}"
        );
        assert!(
            entries.contains("Listener: &tac.Listener{Events: []string{\"Probe\"}, Replay: false, Order: 0, Condition: \"{\\\""),
            "{entries}"
        );
        assert!(entries.contains("Asynchronous: true"), "{entries}");
        assert!(
            entries.contains("Models: \"\", Returns: \"Probe\", Asynchronous: true"),
            "{entries}"
        );
        assert!(
            entries.contains("return instance.Receive(argument0)\n"),
            "{entries}"
        );
    }

    #[test]
    fn events_of_another_type_are_refused() {
        let Err(error) = prepared("OtherService") else {
            panic!("another owner must be refused");
        };
        assert!(
            error
                .to_string()
                .contains("must belong to the page companion"),
            "{error}"
        );
    }
}
