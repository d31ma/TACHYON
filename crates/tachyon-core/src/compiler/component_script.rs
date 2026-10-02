use crate::Failure;
use crate::events::EventModule;
use crate::failure::{diagnostic, source_span};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) struct ComponentScript;

impl ComponentScript {
    const RETIRED_DECORATORS: [&'static str; 3] = ["@onMount", "@publish", "@subscribe"];
    const BEHAVIOR_RUNTIME: &'static str = include_str!("../behavior/runtime.js");

    pub(super) fn prepare_captured(
        snapshot_root: &Path,
        bytes: Vec<u8>,
        source_path: &str,
    ) -> Result<Vec<u8>, Failure> {
        let source = std::str::from_utf8(&bytes)
            .map_err(|_| Self::retired_decorator_failure(source_path))?;
        let assembled = crate::annotation_modules::assemble(
            snapshot_root,
            Path::new(source_path),
            source,
            "tac",
        )?;
        let script = assembled.map_or(bytes, String::into_bytes);
        Self::prepare(snapshot_root, &script, source_path)
    }

    pub(super) fn prepare(
        snapshot_root: &Path,
        bytes: &[u8],
        source_path: &str,
    ) -> Result<Vec<u8>, Failure> {
        let source =
            std::str::from_utf8(bytes).map_err(|_| Self::retired_decorator_failure(source_path))?;
        Self::refuse_retired_decorators(source, source_path)?;
        let path = Path::new(source_path);
        let models = crate::model_sources::ModelSources::read(
            snapshot_root,
            "client",
            path.extension()
                .and_then(|value| value.to_str())
                .unwrap_or(""),
        )?;
        let metadata = models.metadata();
        let assembled = metadata.assembled(path, source)?;
        let source = assembled.as_str();
        crate::behavior::StoredState::refuse_sigils(path, source)?;
        let events = Self::described_events(snapshot_root, path, source)?;
        let resources = events
            .as_ref()
            .map(Self::event_resources)
            .unwrap_or_default();
        let source = events
            .as_ref()
            .map_or(source, |events| events.source.as_str());
        let lowered = super::captured_browser_behavior::CapturedBrowserBehavior::lower(
            snapshot_root,
            path,
            source,
            &resources,
        )?;
        let lowered = lowered
            .map(|text| metadata.without_declarations(path, &text))
            .transpose()?;
        let plain = metadata.without_declarations(path, source)?;
        let prepared = Self::lowered_bytes(lowered, &plain);
        let mut bytes = models
            .add_to_program(&String::from_utf8_lossy(&prepared))
            .into_bytes();
        bytes.extend(Self::event_plan_export(events.as_ref(), source_path)?);
        Ok(bytes)
    }

    fn refuse_retired_decorators(source: &str, source_path: &str) -> Result<(), Failure> {
        if Self::uses_retired_decorator(source) {
            return Err(Self::retired_decorator_failure(source_path));
        }
        Ok(())
    }

    fn described_events(
        snapshot_root: &Path,
        path: &Path,
        source: &str,
    ) -> Result<Option<EventModule>, Failure> {
        let mut events = crate::events::extract(path, source, "tac")?;
        events.iter_mut().try_for_each(|module| {
            crate::events::describe(module, path, snapshot_root, "client")
        })?;
        Ok(events)
    }

    fn lowered_bytes(lowered: Option<String>, source: &str) -> Vec<u8> {
        match lowered {
            Some(mut lowered) => {
                lowered.push('\n');
                lowered.push_str(Self::BEHAVIOR_RUNTIME);
                lowered.into_bytes()
            }
            None => source.as_bytes().to_vec(),
        }
    }

    fn event_plan_export(
        events: Option<&EventModule>,
        source_path: &str,
    ) -> Result<Vec<u8>, Failure> {
        match events {
            Some(events) => Self::event_plan(events, source_path),
            None => Ok(Vec::new()),
        }
    }

    fn event_plan(events: &EventModule, source_path: &str) -> Result<Vec<u8>, Failure> {
        let plan = serde_json::to_string(&serde_json::json!({"types": events.types}))
            .map_err(|_| Self::retired_decorator_failure(source_path))?;
        Ok(format!("\nexport const __tachyon_event_plan__ = {plan};\n").into_bytes())
    }

    fn uses_retired_decorator(source: &str) -> bool {
        let mask = crate::lexical::code_mask("js", source);
        Self::RETIRED_DECORATORS
            .iter()
            .any(|name| Self::used_in_code(source, name, &mask.code))
    }

    fn used_in_code(source: &str, name: &str, code: &[bool]) -> bool {
        source
            .match_indices(name)
            .any(|(position, _)| code.get(position) == Some(&true))
    }

    fn event_resources(events: &EventModule) -> BTreeMap<String, BTreeSet<String>> {
        events
            .types
            .iter()
            .map(|event_type| {
                let fields = event_type
                    .resources
                    .iter()
                    .map(|resource| resource.field.clone())
                    .collect();
                (event_type.name.clone(), fields)
            })
            .collect()
    }

    fn retired_decorator_failure(source_path: &str) -> Failure {
        Failure::one(diagnostic(
            1306,
            "Retired Tac lifecycle and signal decorators are no longer supported.",
            Some(String::from(
                "Use @OnStart/@OnStop for lifecycle, an injected applicationEventPublisher with OnEvent for local events, and @Needs fields named after the owner-bound browser resources.",
            )),
            source_span(source_path, 0, 0),
        ))
    }
}
