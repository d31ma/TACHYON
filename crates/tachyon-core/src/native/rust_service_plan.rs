use crate::{Failure, events::EventModule};
use std::path::{Path, PathBuf};

pub(super) struct RustServicePlan {
    pub(super) path: PathBuf,
    pub(super) name: String,
    pub(super) source: String,
    pub(super) prototype: bool,
    pub(super) required: Vec<String>,
    pub(super) events: Option<EventModule>,
}

impl RustServicePlan {
    pub(super) fn read(path: &Path, source: &str) -> Result<Self, Failure> {
        let roles = crate::behavior::ServiceRoles::read(path, source)?;
        let consumed = crate::behavior::ServiceRoles::without_roles(path, source)?;
        let events = crate::events::extract(path, &consumed, "tac")?;
        let source = events
            .as_ref()
            .map_or(consumed.as_str(), |module| module.source.as_str())
            .to_owned();
        let name = crate::companion::owner_name(
            crate::companion::CompanionLanguage::Rust,
            &source,
            &path.to_string_lossy(),
        )
        .ok_or_else(|| {
            super::host::native_tool_failure(
                1406,
                "Native Rust Service must declare its filename's concrete struct.",
            )
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            name,
            source,
            prototype: roles.prototype,
            required: roles.required,
            events,
        })
    }

    pub(super) fn prepared_events(&self) -> Result<super::events::Prepared, Failure> {
        let events = self.events.clone().map(Self::without_lifecycle);
        let mut prepared = super::events::prepare(
            crate::project::NativeCompanion::Rust,
            &self.source,
            events.as_ref(),
            &self.path,
        )?;
        prepared.excluded.extend(self.hook_names());
        Ok(prepared)
    }

    fn without_lifecycle(mut module: EventModule) -> EventModule {
        Self::remove_lifecycle(&mut module);
        module
    }

    fn remove_lifecycle(module: &mut EventModule) {
        let hooks = module
            .types
            .iter_mut()
            .flat_map(|kind| [kind.post_construct.take(), kind.pre_destroy.take()])
            .flatten()
            .collect::<Vec<_>>();
        module
            .methods
            .retain(|method| !hooks.contains(&method.name));
    }

    fn hook_names(&self) -> Vec<String> {
        self.events
            .iter()
            .flat_map(|module| module.types.iter())
            .flat_map(|kind| [&kind.post_construct, &kind.pre_destroy])
            .flatten()
            .cloned()
            .collect()
    }

    pub(super) fn lifecycle(&self, arc: bool, destroy: bool) -> Result<String, Failure> {
        let hook = self.hook_name(destroy);
        hook.map_or_else(|| Ok(String::from("Ok(())")), |name| self.hook(arc, name))
    }

    fn hook_name(&self, destroy: bool) -> Option<&str> {
        let hook = self
            .events
            .as_ref()
            .and_then(|module| module.types.iter().find(|kind| kind.name == self.name))
            .and_then(|kind| {
                if destroy {
                    kind.pre_destroy.as_ref()
                } else {
                    kind.post_construct.as_ref()
                }
            });
        hook.map(String::as_str)
    }

    fn hook(&self, arc: bool, name: &str) -> Result<String, Failure> {
        let method = self.method(name)?;
        Self::validate_hook(method)?;
        let receiver = Self::receiver(arc);
        let call = format!("{receiver}.{name}()");
        self.hook_result(&call, &method.return_type)
    }

    fn method(&self, name: &str) -> Result<&crate::events::EventMethod, Failure> {
        self.events
            .as_ref()
            .and_then(|module| {
                module.methods.iter().find(|method| {
                    [method.owner.as_str(), method.name.as_str()] == [self.name.as_str(), name]
                })
            })
            .ok_or_else(|| {
                super::host::native_tool_failure(1406, "Native Rust lifecycle method is missing.")
            })
    }

    fn validate_hook(method: &crate::events::EventMethod) -> Result<(), Failure> {
        if method.asynchronous || !method.parameters.is_empty() {
            return Err(super::host::native_tool_failure(
                1406,
                "Native Rust service lifecycle must be synchronous and take no injected arguments.",
            ));
        }
        Ok(())
    }

    fn receiver(arc: bool) -> &'static str {
        if arc { "value" } else { "value.borrow_mut()" }
    }

    fn hook_result(&self, call: &str, kind: &str) -> Result<String, Failure> {
        let kind = kind.split_whitespace().collect::<String>();
        let results = [
            "Result<",
            "std::result::Result<",
            "::std::result::Result<",
            "core::result::Result<",
            "::core::result::Result<",
        ];
        let result = results.iter().any(|prefix| kind.starts_with(prefix));
        match (result, kind.as_str()) {
            (true, _) => Ok(format!(
                "{call}.map(|_| ()).map_err(|_| String::from(\"Native service lifecycle failed.\"))"
            )),
            (false, "" | "()") => Ok(format!("{{ let _ = {call}; Ok(()) }}")),
            _ => Err(self.failure("Native Rust lifecycle must return unit or a supported std/core Result; return aliases and other values need an explicit supported contract.")),
        }
    }

    pub(super) fn failure(&self, message: &str) -> Failure {
        let at = self.source.find(&self.name).unwrap_or(0);
        Failure::one(crate::failure::diagnostic(
            1406,
            message,
            None,
            crate::failure::source_span(&self.path.to_string_lossy(), at, at + self.name.len()),
        ))
    }

    pub(super) fn refuse_unowned_events(&self) -> Result<(), Failure> {
        Self::require_owned_events(self.has_events())
    }

    fn has_events(&self) -> bool {
        self.events
            .iter()
            .flat_map(|module| &module.types)
            .any(|kind| {
                [
                    kind.listeners.is_empty(),
                    kind.resources.is_empty(),
                    kind.streams.is_empty(),
                ] != [true; 3]
            })
    }

    fn require_owned_events(listeners: bool) -> Result<(), Failure> {
        if listeners {
            return Err(super::host::native_tool_failure(
                1406,
                "Native peer Service event subscriptions require an exposed activation owner; only peer lifecycle hooks are supported by the page graph.",
            ));
        }
        Ok(())
    }
}
