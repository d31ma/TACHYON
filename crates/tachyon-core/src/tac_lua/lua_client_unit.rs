use super::{LuaConverterProviders, LuaWebServices};
use crate::Failure;
use crate::behavior::LuaBehaviorPlan;
use crate::failure::{diagnostic, source_span};
use crate::lua_source::{LuaBehaviorEdits, LuaSource, LuaTestUnit, LuaView};
use crate::model_sources::ModelSources;
use std::path::Path;

pub(crate) struct LuaClientUnit;

impl LuaClientUnit {
    const REALM: &'static str = "tac";
    const CLASS: &'static str = "---@class ";
    const HOST: &'static str = "TachyonHost = TachyonHost or {\n  test_unit = true,\n  emit = function() end,\n  ask = function(text) return YonJson.encode(BrowserFakes.reply(YonJson.decode(text))) end,\n}\n";
    const LOCAL_CLOCK: [&'static str; 2] = ["", "BehaviorClock.host = nil\n"];
    const NO_UNIT: &'static str =
        "A test unit declares Test on one class; this source declares none.";

    pub(crate) fn program(
        project: &Path,
        relative: &Path,
        source: &str,
    ) -> Result<String, Failure> {
        let assembled =
            crate::annotation_modules::assemble(project, relative, source, Self::REALM)?
                .unwrap_or_else(|| source.to_owned());
        let view = LuaView::joined(relative, &assembled);
        let mapped = |failure: Failure| view.failure(&failure);
        let roles = Self::roles(&view)?;
        let units = crate::behavior::unit_tables(view.path(), &roles.source, Self::REALM)
            .map_err(mapped)?;
        let unit = units
            .first()
            .ok_or_else(|| Self::failure(relative, source))?;
        let assembled = ModelSources::client_metadata(project, LuaSource::EXTENSION)?
            .assembled(relative, &assembled)?;
        let joined = LuaView::joined(relative, &assembled);
        let joined_failure = |failure: Failure| joined.failure(&failure);
        let joined_roles = Self::roles(&joined)?;
        let plan = LuaBehaviorPlan::read(joined.path(), &joined_roles.source, Self::REALM)
            .map_err(joined_failure)?;
        let contracts = plan
            .as_ref()
            .map(|plan| Self::contracts(plan, &assembled))
            .transpose()
            .map_err(|message| Self::lowering_failure(relative, source, &message))?
            .unwrap_or_default();
        let authored = plan
            .as_ref()
            .map(|plan| LuaBehaviorEdits::tac(plan).tac_program(relative, &assembled))
            .transpose()
            .map_err(|message| Self::lowering_failure(relative, source, &message))?
            .unwrap_or(assembled);
        let body = format!(
            "{}{}",
            LuaWebServices::model_names(project)?,
            LuaTestUnit::new(unit)
                .with_behaviors(plan.is_some())
                .tac_program(&format!("{authored}\n{contracts}"))
        );
        Ok(format!(
            "{}{}{}\n{}{}",
            LuaWebServices::PRELUDE,
            Self::HOST,
            LuaWebServices::behavior_runtime(plan.is_some()),
            Self::LOCAL_CLOCK[usize::from(plan.is_some())],
            LuaWebServices::client_models(project)?.add_to_program(&body)
        ))
    }

    fn roles(view: &LuaView) -> Result<crate::behavior::ServiceRoles, Failure> {
        let mapped = |failure: Failure| view.failure(&failure);
        let clients =
            crate::behavior::HttpClientSource::rewrite(view.path(), view.text()).map_err(mapped)?;
        crate::behavior::ServiceRoles::read(view.path(), clients.as_deref().unwrap_or(view.text()))
            .map_err(mapped)
    }

    fn contracts(plan: &LuaBehaviorPlan, source: &str) -> Result<String, String> {
        source
            .lines()
            .filter_map(|line| line.strip_prefix(Self::CLASS))
            .filter_map(|rest| rest.split_whitespace().next())
            .map(|class| LuaConverterProviders::written_when_bound(class, source, plan))
            .collect()
    }

    fn lowering_failure(relative: &Path, source: &str, message: &str) -> Failure {
        let portable = relative.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            2015,
            format!("Cannot build the Lua test unit '{portable}': {message}"),
            None,
            source_span(&portable, 0, source.len()),
        ))
    }

    fn failure(relative: &Path, source: &str) -> Failure {
        let portable = relative.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            2015,
            Self::NO_UNIT,
            None,
            source_span(&portable, 0, source.len()),
        ))
    }
}
