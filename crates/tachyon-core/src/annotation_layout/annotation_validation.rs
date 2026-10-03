use super::{AuthoredDomGuard, FolderRoleCheck, Input};
use crate::Failure;
use crate::lua_source::LuaView;
use std::path::Path;

pub(crate) struct AnnotationValidation;

impl AnnotationValidation {
    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        AuthoredDomGuard::check(path, source)?;
        Self::check_layout(path, source)
    }

    pub(crate) fn check_verified_vendor(path: &Path, source: &str) -> Result<(), Failure> {
        Self::check_layout(path, source)
    }

    fn check_layout(path: &Path, source: &str) -> Result<(), Failure> {
        crate::fsharp_source::FsharpStyle::check(path, source)?;
        crate::lua_source::LuaStyle::validate(path, source)?;
        crate::lua_source::LuaStyle::check_rules(path, source)?;
        LuaView::either(path, source, |path, source| {
            FolderRoleCheck::check_source(path, source)?;
            Input::validate_source(path, source)
        })
    }
}
