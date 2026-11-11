use super::{EventModule, EventType};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::route_types::{WebEventModels, WebEventSender};
use std::path::Path;

pub(crate) struct BuiltInNameClash<'a> {
    path: &'a Path,
    root: &'a Path,
    own: &'a [String],
}

impl<'a> BuiltInNameClash<'a> {
    pub(crate) const CODE: u16 = 1376;
    const HELP: &'static str = "Rename your model in client/models, and the parameters that use it, for example to 'NoteUploadFinished'. An app model wins over a built-in event model of the same name, so the build leaves the built-in type out. But here the built-in event can also reach this listener, so the two would mix (ADR 0065).";

    pub(crate) fn new(path: &'a Path, root: &'a Path, own: &'a [String]) -> Self {
        Self { path, root, own }
    }

    pub(crate) fn check(&self, module: &EventModule) -> Result<(), Failure> {
        self.check_types(&module.types)
    }

    fn check_types(&self, types: &[EventType]) -> Result<(), Failure> {
        for event_type in types {
            self.check_type(event_type)?;
        }
        Ok(())
    }

    fn check_type(&self, event_type: &EventType) -> Result<(), Failure> {
        for listener in &event_type.listeners {
            let model = listener.events.first().cloned().unwrap_or_default();
            let place = format!("{}.{}", event_type.name, listener.method);
            self.check_listener(event_type, &model, &place)?;
        }
        Ok(())
    }

    fn check_listener(
        &self,
        event_type: &EventType,
        model: &str,
        place: &str,
    ) -> Result<(), Failure> {
        let sender = self.shadowed_sender(model);
        let reaching = sender.filter(|sender| self.reaches(*sender, event_type));
        reaching.map_or(Ok(()), |sender| Err(self.failure(model, place, sender)))
    }

    fn shadowed_sender(&self, model: &str) -> Option<WebEventSender> {
        if !self.own.iter().any(|name| name == model) {
            return None;
        }
        WebEventModels::sender(model)
    }

    fn reaches(&self, sender: WebEventSender, event_type: &EventType) -> bool {
        match sender {
            WebEventSender::Page => true,
            WebEventSender::Grant(api) => self.granted().iter().any(|grant| grant == api),
            WebEventSender::Socket => event_type.streams.iter().any(|stream| stream.socket),
        }
    }

    fn granted(&self) -> Vec<String> {
        crate::native::SiteSettings::load(self.root)
            .map(|settings| settings.browser_apis)
            .unwrap_or_default()
    }

    fn failure(&self, model: &str, place: &str, sender: WebEventSender) -> Failure {
        let message = format!(
            "The model '{model}' in client/models has the name of a built-in event model, and the OnEvent method '{place}' can also hear the built-in '{model}' event: {}",
            sender.reason()
        );
        let span = source_span(&self.path.to_string_lossy(), 0, 1);
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(Self::HELP.into()),
            span,
        ))
    }
}
