use super::LuaWebServices;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::lua_source::{LuaTestUnit, LuaView};
use std::path::Path;

pub(crate) struct LuaClientUnit;

impl LuaClientUnit {
    const REALM: &'static str = "tac";
    const HOST: &'static str = "TachyonHost = TachyonHost or {\n  emit = function() end,\n  ask = function(text) return YonJson.encode(BrowserFakes.reply(YonJson.decode(text))) end,\n}\n";
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
        let roles =
            crate::behavior::ServiceRoles::read(view.path(), view.text()).map_err(mapped)?;
        let units = crate::behavior::unit_tables(view.path(), &roles.source, Self::REALM)
            .map_err(mapped)?;
        let unit = units
            .first()
            .ok_or_else(|| Self::failure(relative, source))?;
        let body = format!(
            "{}{}",
            LuaWebServices::model_names(project)?,
            LuaTestUnit::new(unit).tac_program(&assembled)
        );
        Ok(format!(
            "{}{}\n{}",
            LuaWebServices::PRELUDE,
            Self::HOST,
            LuaWebServices::client_models(project)?.add_to_program(&body)
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
