use super::{LuaLowering, LuaTestUnit, LuaView};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::yon::Lowered;
use crate::yon::parser::Parsed;
use std::path::Path;

pub(crate) struct LuaYon;

impl LuaYon {
    const LATER: &'static str = "Lua classes do not support {} yet (ADR 0085). Write this class in another Yon language, or leave the annotation out.";
    const STREAMS: &'static str = "streams and sockets (StreamRoute, SocketRoute)";
    const EVENTS: &'static str = "local events and lifecycle hooks (OnEvent, OnStart, OnStop)";
    const BEHAVIORS: &'static str = "behavior annotations (for example Cached, Retry or Transactional)";

    pub(crate) fn lower(source: &Path, contents: &str) -> Result<Option<Lowered>, Failure> {
        let view = LuaView::joined(source, contents);
        let mapped = |failure: Failure| view.failure(&failure);
        let is_test = crate::behavior::test_configuration_name(view.path(), view.text(), "yon")
            .map_err(mapped)?
            .is_some();
        if is_test {
            return Self::test_unit(source, contents, &view);
        }
        Self::refuse_later(source, contents, &view)?;
        let parsed = crate::yon::parser::parse(view.path(), view.text()).map_err(mapped)?;
        let Some(parsed) = parsed else {
            return Ok(None);
        };
        Self::routes(source, contents, &parsed).map(Some)
    }

    fn test_unit(source: &Path, contents: &str, view: &LuaView) -> Result<Option<Lowered>, Failure> {
        let mapped = |failure: Failure| view.failure(&failure);
        let behaviors = crate::behavior::lower(view.path(), view.text(), "yon").map_err(mapped)?;
        let text = behaviors.as_deref().unwrap_or(view.text());
        let Some(parsed) = crate::yon::parser::parse_test(view.path(), text).map_err(mapped)? else {
            return Ok(None);
        };
        let units = crate::behavior::unit_tables(view.path(), view.text(), "yon").map_err(mapped)?;
        let Some(unit) = units.first() else {
            return Ok(None);
        };
        let lowering = LuaLowering::new(&parsed.plan);
        let program = LuaTestUnit::new(unit)
            .program(contents, &lowering)
            .map_err(|message| Self::failure(source, contents, &message))?;
        Ok(Some(Lowered {
            source: program,
            routes: Vec::new(),
            topics: crate::events::TopicUse::default(),
        }))
    }

    fn refuse_later(source: &Path, contents: &str, view: &LuaView) -> Result<(), Failure> {
        let mapped = |failure: Failure| view.failure(&failure);
        let events = crate::events::extract(view.path(), view.text(), "yon").map_err(mapped)?;
        if events.is_some() {
            return Err(Self::later(source, contents, Self::EVENTS));
        }
        let behaviors = crate::behavior::lower(view.path(), view.text(), "yon").map_err(mapped)?;
        if behaviors.is_some() {
            return Err(Self::later(source, contents, Self::BEHAVIORS));
        }
        Ok(())
    }

    fn routes(source: &Path, contents: &str, parsed: &Parsed) -> Result<Lowered, Failure> {
        let streams = parsed
            .plan
            .routes
            .iter()
            .any(|route| route.streaming || route.socket.is_some());
        if streams {
            return Err(Self::later(source, contents, Self::STREAMS));
        }
        let program = LuaLowering::new(&parsed.plan)
            .program(contents)
            .map_err(|message| Self::failure(source, contents, &message))?;
        let routes = parsed
            .plan
            .routes
            .iter()
            .filter(|route| route.pattern.is_some())
            .cloned()
            .collect();
        Ok(Lowered {
            source: program,
            routes,
            topics: crate::events::TopicUse::default(),
        })
    }

    fn later(source: &Path, contents: &str, feature: &str) -> Failure {
        Self::failure(source, contents, &Self::LATER.replace("{}", feature))
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
