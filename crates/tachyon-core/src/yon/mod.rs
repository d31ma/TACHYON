mod compiled;
pub(crate) mod dynamic;
mod error_calls;
pub(crate) mod go_container;
mod go_stream_result;
pub(crate) use go_stream_result::GoStreamResult;
mod manual_helpers;
pub(crate) use manual_helpers::ManualHelpers;
pub(crate) mod parser;
pub(crate) mod plan;
mod required_args;
pub(crate) mod route_access;
pub(crate) mod route_calls;

pub(crate) use required_args::prepare as prepare_required_args;
pub(crate) use required_args::required_fields;

use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::groovy_source::GroovySource;
use std::path::Path;

pub(crate) struct Lowered {
    pub source: String,
    pub routes: Vec<plan::Route>,
    pub topics: crate::events::TopicUse,
}

pub(crate) fn lower_with_routes(
    source: &Path,
    contents: &str,
    models_root: Option<&Path>,
) -> Result<Option<Lowered>, Failure> {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let marked = GroovySource::with_java_part(extension, contents);
    let contents = marked.as_deref().unwrap_or(contents);
    let extension = GroovySource::syntax(extension);
    let manual_helpers = manual_helpers::ManualHelpers::read(source, contents)?;
    let mut event_module = crate::events::extract(source, contents, "yon")?;
    if let (Some(module), Some(root)) = (event_module.as_mut(), models_root) {
        crate::events::describe(module, source, root, "server")?;
    }
    let contents = event_module
        .as_ref()
        .map_or(contents, |module| module.source.as_str());
    let prepared = required_args::prepare_with_manual(source, contents, &manual_helpers)?;
    let contents = prepared.as_deref().unwrap_or(contents);
    let is_test = crate::behavior::test_configuration_name(source, contents, "yon")?.is_some();
    let behavior_source = crate::behavior::lower(source, contents, "yon")?;
    let contents = behavior_source.as_deref().unwrap_or(contents);
    let parsed = if is_test {
        parser::parse_test_prepared(source, contents, &manual_helpers)?
    } else {
        parser::parse_prepared(source, contents, None, &manual_helpers)?
    };
    let Some(parsed) = parsed else {
        if event_module.is_some() {
            return Err(Failure::one(diagnostic(
                2015,
                "Yon local events and lifecycle hooks require managed provider roles",
                None,
                None,
            )));
        }
        let lowered = behavior_source.map(|source| with_behavior_runtime(source, extension));
        return Ok(lowered.map(|source| Lowered {
            source,
            routes: Vec::new(),
            topics: crate::events::TopicUse::default(),
        }));
    };
    let lowered = lower_parsed(source, extension, &parsed, event_module.as_ref())
        .map_err(|message| lowering_failure(source, contents, &message))?;
    let lowered = if behavior_source.is_some() {
        with_behavior_runtime(lowered, extension)
    } else {
        lowered
    };
    let mut routes = Vec::new();
    for route in parsed.plan.routes {
        if route.pattern.is_some() {
            routes.push(route);
        }
    }
    let topics = event_module
        .as_ref()
        .map(crate::events::EventModule::topic_use)
        .unwrap_or_default();
    Ok(Some(Lowered {
        source: lowered,
        routes,
        topics,
    }))
}

pub(crate) fn lower_gateway_unit(
    source: &Path,
    contents: &str,
    target: &str,
    methods: &std::collections::BTreeMap<String, usize>,
) -> Result<String, Failure> {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let marked = GroovySource::with_java_part(extension, contents);
    let contents = marked.as_deref().unwrap_or(contents);
    let extension = GroovySource::syntax(extension);
    let portable = source.to_string_lossy().replace('\\', "/");
    let manual_helpers = manual_helpers::ManualHelpers::read(source, contents)?;
    if crate::events::extract(source, contents, "yon")?.is_some() {
        return Err(crate::behavior::gateway_target_failure(
            &portable,
            0,
            "A gateway target language whose role files use local events or lifecycle hooks cannot host a gateway unit yet",
        ));
    }
    let prepared = required_args::prepare_with_manual(source, contents, &manual_helpers)?;
    let contents = prepared.as_deref().unwrap_or(contents);
    let behavior_source = crate::behavior::lower(source, contents, "yon")?;
    let contents = behavior_source.as_deref().unwrap_or(contents);
    let Some(parsed) = parser::parse_prepared(source, contents, Some(target), &manual_helpers)?
    else {
        return Err(crate::behavior::gateway_target_failure(
            &portable,
            0,
            &format!("Gateway target {target} is not a managed Yon type"),
        ));
    };
    let lowered = match extension {
        "js" | "ts" | "py" | "php" | "rb" => {
            dynamic::gateway(extension, &parsed.source, &parsed.plan, methods)
        }
        _ => compiled::gateway(extension, &parsed.source, &parsed.plan, methods),
    }
    .map_err(|message| lowering_failure(source, contents, &message))?;
    if behavior_source.is_some() {
        return Ok(with_behavior_runtime(lowered, extension));
    }
    Ok(lowered)
}

fn lower_parsed(
    source: &Path,
    extension: &str,
    parsed: &parser::Parsed,
    events: Option<&crate::events::EventModule>,
) -> Result<String, String> {
    if let Some(socket) = parsed
        .plan
        .routes
        .iter()
        .find_map(|route| route.socket.as_ref())
    {
        if events.is_some() {
            return Err(String::from(
                "A @SocketRoute class cannot hold @OnEvent, @OnStart or @OnStop methods. Put them in a @Service and give the socket class a @Needs field for it",
            ));
        }
        return dynamic::socket(extension, &parsed.source, &parsed.plan, socket);
    }
    if !route_calls::RouteCalls::present(&parsed.plan) {
        return lower_plan(source, extension, &parsed.source, &parsed.plan, events);
    }
    let functions = route_calls::RouteCalls::functions(extension, &parsed.plan, &parsed.source)?;
    let error_functions =
        error_calls::ErrorCalls::functions(extension, &parsed.plan, &parsed.source)?;
    let verb_plan = route_calls::RouteCalls::verb_plan(&parsed.plan, extension)?;
    let lowered = lower_plan(source, extension, &parsed.source, &verb_plan, events)?;
    Ok(format!("{lowered}\n{functions}\n{error_functions}"))
}

fn lower_plan(
    source: &Path,
    extension: &str,
    stripped: &str,
    plan: &plan::Plan,
    events: Option<&crate::events::EventModule>,
) -> Result<String, String> {
    if let Some(events) = events {
        return crate::events::yon::lower(source, stripped, plan, events);
    }
    match extension {
        "js" | "ts" | "py" | "php" | "rb" => dynamic::lower(extension, stripped, plan),
        _ => compiled::lower(extension, stripped, plan),
    }
}

fn lowering_failure(source: &Path, contents: &str, message: &str) -> Failure {
    let portable = source.to_string_lossy().replace('\\', "/");
    Failure::one(diagnostic(
        2015,
        format!("Cannot lower Yon annotations in '{portable}': {message}"),
        Some(String::from(
            "See docs/YON_ANNOTATIONS.md for supported declaration forms.",
        )),
        source_span(&portable, 0, contents.len()),
    ))
}

fn with_behavior_runtime(mut source: String, extension: &str) -> String {
    if !matches!(extension, "js" | "ts") {
        return source;
    }
    source.push('\n');
    source.push_str(include_str!("../behavior/runtime.js"));
    source
}
