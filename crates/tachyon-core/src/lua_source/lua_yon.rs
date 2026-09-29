use super::{LuaLowering, LuaTestUnit, LuaView};
use crate::Failure;
use crate::behavior::UnitTable;
use crate::failure::{diagnostic, source_span};
use crate::yon::Lowered;
use crate::yon::parser::Parsed;
use crate::yon::plan::{Plan, Route};
use std::path::Path;

type Lowering = fn(&Path, &str, &LuaView) -> Result<Option<Lowered>, Failure>;

pub(crate) struct LuaYon;

impl LuaYon {
    const LATER: &'static str = "Lua classes do not support {} yet (ADR 0085). Write this class in another Yon language, or leave the annotation out.";
    const STREAMS: &'static str = "streams and sockets (StreamRoute, SocketRoute)";
    const EVENTS: &'static str = "local events and lifecycle hooks (OnEvent, OnStart, OnStop)";
    const BEHAVIORS: &'static str =
        "behavior annotations (for example Cached, Retry or Transactional)";
    const REALM: &'static str = "yon";
    const LOWERINGS: [Lowering; 2] = [Self::roles, Self::test_unit];

    pub(crate) fn lower(source: &Path, contents: &str) -> Result<Option<Lowered>, Failure> {
        let view = LuaView::joined(source, contents);
        let is_test =
            crate::behavior::test_configuration_name(view.path(), view.text(), Self::REALM)
                .map_err(|failure| view.failure(&failure))?
                .is_some();
        Self::LOWERINGS[usize::from(is_test)](source, contents, &view)
    }

    fn roles(source: &Path, contents: &str, view: &LuaView) -> Result<Option<Lowered>, Failure> {
        Self::refuse_later(source, contents, view)?;
        let parsed = crate::yon::parser::parse(view.path(), view.text())
            .map_err(|failure| view.failure(&failure))?;
        parsed
            .map(|parsed| Self::routes(source, contents, &parsed))
            .transpose()
    }

    fn test_unit(
        source: &Path,
        contents: &str,
        view: &LuaView,
    ) -> Result<Option<Lowered>, Failure> {
        let mapped = |failure: Failure| view.failure(&failure);
        let behaviors =
            crate::behavior::lower(view.path(), view.text(), Self::REALM).map_err(mapped)?;
        let text = behaviors.as_deref().unwrap_or(view.text());
        let parsed = crate::yon::parser::parse_test(view.path(), text).map_err(mapped)?;
        let units =
            crate::behavior::unit_tables(view.path(), view.text(), Self::REALM).map_err(mapped)?;
        parsed
            .zip(units.first())
            .map(|(parsed, unit)| Self::unit_program(source, contents, &parsed.plan, unit))
            .transpose()
    }

    fn unit_program(
        source: &Path,
        contents: &str,
        plan: &Plan,
        unit: &UnitTable,
    ) -> Result<Lowered, Failure> {
        let lowering = LuaLowering::new(plan);
        let program = LuaTestUnit::new(unit)
            .program(contents, &lowering)
            .map_err(|message| Self::failure(source, contents, &message))?;
        Ok(Lowered {
            source: program,
            routes: Vec::new(),
            topics: crate::events::TopicUse::default(),
        })
    }

    fn refuse_later(source: &Path, contents: &str, view: &LuaView) -> Result<(), Failure> {
        let mapped = |failure: Failure| view.failure(&failure);
        let events =
            crate::events::extract(view.path(), view.text(), Self::REALM).map_err(mapped)?;
        Self::refused(events.is_some(), source, contents, Self::EVENTS)?;
        let behaviors =
            crate::behavior::lower(view.path(), view.text(), Self::REALM).map_err(mapped)?;
        Self::refused(behaviors.is_some(), source, contents, Self::BEHAVIORS)
    }

    fn refused(used: bool, source: &Path, contents: &str, feature: &str) -> Result<(), Failure> {
        used.then(|| Self::failure(source, contents, &Self::LATER.replace("{}", feature)))
            .map_or(Ok(()), Err)
    }

    fn routes(source: &Path, contents: &str, parsed: &Parsed) -> Result<Lowered, Failure> {
        Self::refused(Self::streams(&parsed.plan), source, contents, Self::STREAMS)?;
        let program = LuaLowering::new(&parsed.plan)
            .program(contents)
            .map_err(|message| Self::failure(source, contents, &message))?;
        Ok(Lowered {
            source: program,
            routes: Self::patterned(&parsed.plan),
            topics: crate::events::TopicUse::default(),
        })
    }

    fn streams(plan: &Plan) -> bool {
        plan.routes.iter().any(Self::is_stream)
    }

    fn is_stream(route: &Route) -> bool {
        [route.streaming, route.socket.is_some()].contains(&true)
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
