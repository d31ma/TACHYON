use super::{
    LuaBehaviorEdits, LuaInput, LuaLowering, LuaResources, LuaSockets, LuaTestUnit, LuaView,
};
use crate::Failure;
use crate::behavior::{HttpClientSource, LuaBehaviorPlan};
use crate::events::EventModule;
use crate::failure::{diagnostic, source_span};
use crate::yon::Lowered;
use crate::yon::parser::Parsed;
use crate::yon::plan::{Plan, Route};
use std::path::Path;

type Lowering = fn(&LuaInput<'_>) -> Result<Option<Lowered>, Failure>;

pub(crate) struct LuaYon;

impl LuaYon {
    const SOCKET_EVENTS: &'static str = "A @SocketRoute class cannot hold @OnEvent, @OnStart or @OnStop methods. Put them in a @Service and give the socket class a @Needs field for it";
    const REALM: &'static str = "yon";
    const MODELS: &'static str = "server";
    const LOWERINGS: [Lowering; 2] = [Self::roles, Self::test_unit];

    pub(crate) fn lower(
        source: &Path,
        contents: &str,
        models_root: Option<&Path>,
    ) -> Result<Option<Lowered>, Failure> {
        let view = LuaView::joined(source, contents);
        let is_test =
            crate::behavior::test_configuration_name(view.path(), view.text(), Self::REALM)
                .map_err(|failure| view.failure(&failure))?
                .is_some();
        let input = LuaInput {
            source,
            contents,
            view: &view,
            models_root,
        };
        Self::LOWERINGS[usize::from(is_test)](&input)
    }

    fn roles(input: &LuaInput<'_>) -> Result<Option<Lowered>, Failure> {
        let view = input.view;
        let mapped = |failure: Failure| view.failure(&failure);
        let events = Self::events(input)?;
        let text = events
            .as_ref()
            .map_or(view.text(), |module| module.source.as_str());
        let clients = HttpClientSource::rewrite(view.path(), text).map_err(mapped)?;
        let text = clients.as_deref().unwrap_or(text);
        let behaviors = LuaBehaviorPlan::read(view.path(), text, Self::REALM).map_err(mapped)?;
        let erased = behaviors.as_ref().map_or(text, |plan| plan.erased.as_str());
        let parsed = crate::yon::parser::parse(view.path(), erased).map_err(mapped)?;
        let authored = Self::with_behaviors(input, behaviors.as_ref())?;
        parsed
            .map(|parsed| Self::routes(input, &authored, &parsed, events.as_ref()))
            .transpose()
    }

    fn with_behaviors(
        input: &LuaInput<'_>,
        behaviors: Option<&LuaBehaviorPlan>,
    ) -> Result<String, Failure> {
        behaviors.map_or_else(
            || Ok(input.contents.to_owned()),
            |plan| {
                LuaBehaviorEdits::new(plan)
                    .program(input.source, input.contents)
                    .map_err(|message| Self::failure(input.source, input.contents, &message))
            },
        )
    }

    fn events(input: &LuaInput<'_>) -> Result<Option<EventModule>, Failure> {
        let view = input.view;
        let mut events = crate::events::extract(view.path(), view.text(), Self::REALM)
            .map_err(|failure| view.failure(&failure))?;
        if let Some((module, root)) = events.as_mut().zip(input.models_root) {
            crate::events::describe(module, input.source, root, Self::MODELS)?;
        }
        Ok(events)
    }

    fn test_unit(input: &LuaInput<'_>) -> Result<Option<Lowered>, Failure> {
        let view = input.view;
        let mapped = |failure: Failure| view.failure(&failure);
        let clients = HttpClientSource::rewrite(view.path(), view.text()).map_err(mapped)?;
        let behaviors = LuaBehaviorPlan::read(
            view.path(),
            clients.as_deref().unwrap_or(view.text()),
            Self::REALM,
        )
        .map_err(mapped)?;
        let text = behaviors
            .as_ref()
            .map_or(view.text(), |plan| plan.erased.as_str());
        let parsed = crate::yon::parser::parse_test(view.path(), text).map_err(mapped)?;
        let units =
            crate::behavior::unit_tables(view.path(), view.text(), Self::REALM).map_err(mapped)?;
        let authored = Self::with_behaviors(input, behaviors.as_ref())?;
        let unit_behaviors = Self::unit_behaviors(behaviors.as_ref());
        parsed
            .zip(units.first())
            .map(|(parsed, unit)| {
                Self::unit_program(
                    input,
                    &authored,
                    &parsed.plan,
                    &LuaTestUnit::new(unit).with_behaviors(unit_behaviors),
                )
            })
            .transpose()
    }

    fn unit_behaviors(behaviors: Option<&LuaBehaviorPlan>) -> bool {
        behaviors.is_some()
    }

    fn unit_program(
        input: &LuaInput<'_>,
        authored: &str,
        plan: &Plan,
        unit: &LuaTestUnit<'_>,
    ) -> Result<Lowered, Failure> {
        LuaResources::refuse(plan, input.source, input.contents)
            .map_err(|message| Self::failure(input.source, input.contents, &message))?;
        let lowering = LuaLowering::new(plan);
        let program = unit
            .program(authored, &lowering)
            .map_err(|message| Self::failure(input.source, input.contents, &message))?;
        Ok(Lowered {
            source: program,
            routes: Vec::new(),
            topics: crate::events::TopicUse::default(),
        })
    }

    fn routes(
        input: &LuaInput<'_>,
        authored: &str,
        parsed: &Parsed,
        events: Option<&EventModule>,
    ) -> Result<Lowered, Failure> {
        let (source, contents) = (input.source, input.contents);
        LuaResources::refuse(&parsed.plan, source, contents)
            .map_err(|message| Self::failure(source, contents, &message))?;
        let program = Self::program(&parsed.plan, authored, events)
            .map_err(|message| Self::failure(source, contents, &message))?;
        Ok(Lowered {
            source: program,
            routes: Self::patterned(&parsed.plan),
            topics: events.map(EventModule::topic_use).unwrap_or_default(),
        })
    }

    fn program(
        plan: &Plan,
        contents: &str,
        events: Option<&EventModule>,
    ) -> Result<String, String> {
        let socket = plan.routes.iter().find_map(|route| route.socket.as_ref());
        match (socket, events) {
            (Some(_), Some(_)) => Err(String::from(Self::SOCKET_EVENTS)),
            (Some(methods), None) => LuaSockets::new(plan, methods).program(contents),
            (None, Some(events)) => LuaLowering::evented_program(plan, contents, events),
            (None, None) => LuaLowering::new(plan).program(contents),
        }
    }

    fn patterned(plan: &Plan) -> Vec<Route> {
        plan.routes
            .iter()
            .filter(|route| route.pattern.is_some())
            .cloned()
            .collect()
    }

    fn failure(source: &Path, contents: &str, message: &str) -> Failure {
        let portable = source.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            2015,
            format!("Cannot lower Yon annotations in '{portable}': {message}"),
            None,
            source_span(&portable, 0, contents.len()),
        ))
    }
}
