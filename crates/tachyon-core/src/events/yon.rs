use super::{EventMethod, EventModule};
use crate::yon::plan::{Plan, Scope};
use std::fmt::Write;
use std::path::Path;

mod containers;

struct EventLowering;

impl EventLowering {
    const ASYNC_RETURN_TOKENS: [&str; 5] =
        ["Task", "Future", "CompletionStage", "Sequence", "Iterator"];
}

pub(crate) fn lower(
    path: &Path,
    source: &str,
    plan: &Plan,
    events: &EventModule,
) -> Result<String, String> {
    let extension = crate::groovy_source::GroovySource::syntax(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or(""),
    );
    let has_async_methods = is_async_language(extension);
    if !has_async_methods && events.methods.iter().any(is_asynchronous_method) {
        return Err(
            "This Yon language supports synchronous local event and lifecycle completion only"
                .into(),
        );
    }
    if !has_async_methods && has_async_listener(events) {
        return Err(
            "InBackground listeners require a Yon language with asynchronous methods (js, ts, py)"
                .into(),
        );
    }
    validate_providers(plan, events)?;
    if plan.routes.iter().any(|route| route.streaming) {
        return Err("Local event containers do not support streaming endpoints; use a separate non-streaming event controller".into());
    }
    let relocated = super::relocate(path, source, events)
        .map_err(|error| format!("Cannot locate event methods: {error:?}"))?;
    let source = relocated.source.clone();
    let container = containers::lower(extension, plan, events)?;
    let mut runtime = String::new();
    if extension == "go" {
        runtime.push_str(crate::go_source::GoFile::MARKER);
        runtime.push_str("events\n");
    }
    runtime.push_str(runtime_source(extension)?);
    Ok(format!("{source}\n{runtime}\n{container}"))
}

fn is_async_language(extension: &str) -> bool {
    matches!(extension, "js" | "ts" | "py")
}

fn is_asynchronous_method(method: &EventMethod) -> bool {
    if method.asynchronous {
        return true;
    }
    EventLowering::ASYNC_RETURN_TOKENS
        .iter()
        .any(|token| method.return_type.contains(token))
}

fn has_async_listener(events: &EventModule) -> bool {
    events.types.iter().any(|event_type| {
        event_type
            .listeners
            .iter()
            .any(|listener| listener.asynchronous)
    })
}

fn validate_providers(plan: &Plan, events: &EventModule) -> Result<(), String> {
    let has_transient_provider = plan
        .providers
        .iter()
        .any(|provider| provider.scope == Scope::Transient);
    if has_transient_provider {
        return Err(
            "Local event provider graphs require invocation-cached providers, not transient scope"
                .into(),
        );
    }
    for event_type in &events.types {
        let Some(provider) = plan
            .providers
            .iter()
            .find(|provider| provider.name == event_type.name)
        else {
            return Err(format!(
                "Event owner {} needs a managed Yon role",
                event_type.name
            ));
        };
        if provider.scope == Scope::Transient {
            return Err(
                "Event/lifecycle providers must be invocation-cached, not transient".into(),
            );
        }
    }
    Ok(())
}

fn runtime_source(extension: &str) -> Result<&'static str, String> {
    match extension {
        "js" | "ts" => Ok(include_str!("yon/javascript.js")),
        "py" => Ok(include_str!("yon/python.py")),
        "php" => Ok(include_str!("yon/php.php")),
        "java" => Ok(include_str!("yon/java.java")),
        "kt" => Ok(include_str!("yon/kotlin.kt")),
        "cs" => Ok(include_str!("yon/csharp.cs")),
        "rs" => Ok(include_str!("yon/rust.rs")),
        "dart" => Ok(include_str!("yon/dart.dart")),
        "go" => Ok(include_str!("yon/go.go")),
        "rb" => Ok(include_str!("yon/ruby.rb")),
        _ => Err("No Yon event runtime for this language".into()),
    }
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn index(plan: &Plan, name: &str) -> Result<usize, String> {
    plan.providers
        .iter()
        .position(|provider| provider.name == name)
        .ok_or_else(|| format!("Missing event provider {name}"))
}
