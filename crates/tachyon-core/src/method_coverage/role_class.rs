use super::{FsharpMembers, PublicMethod, PublicMethods, RoleFiles};
use crate::Failure;
use crate::annotation_layout::UtilityLayout;
use crate::fsharp_source::FsharpView;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct RoleClass {
    pub(crate) file: String,
    pub(crate) realm: String,
    pub(crate) language: String,
    pub(crate) type_name: String,
    pub(crate) masked: String,
    pub(crate) methods: Vec<PublicMethod>,
}

impl RoleClass {
    const STATIC_WORD: &'static str = "static ";
    const STATIC_BLANK: &'static str = "       ";

    pub(crate) fn read(relative: &Path, source: &str) -> Result<Option<Self>, Failure> {
        let utility = UtilityLayout::is_utility(relative);
        let type_name = Self::type_name(relative, source, utility)?;
        type_name
            .filter(|_| RoleFiles::contains(relative))
            .map(|type_name| Self::with_type(relative, source, utility, type_name))
            .transpose()
    }

    pub(crate) fn key(&self, method: &str) -> String {
        format!("{}.{method}", self.type_name)
    }

    fn type_name(relative: &Path, source: &str, utility: bool) -> Result<Option<String>, Failure> {
        let mut types = crate::annotation_layout::behaviour_types(relative, source)?;
        types.extend(Self::utility_name(relative, utility));
        Ok(types.into_iter().next())
    }

    fn utility_name(relative: &Path, utility: bool) -> Option<String> {
        relative
            .file_stem()
            .filter(|_| utility)
            .map(|stem| stem.to_string_lossy().into_owned())
    }

    fn with_type(
        relative: &Path,
        source: &str,
        utility: bool,
        type_name: String,
    ) -> Result<Self, Failure> {
        let masked = Self::masked(relative, source, utility)?;
        let language = Self::text(relative.extension());
        let methods = Self::methods(relative, source, utility, &language, &masked)?;
        Ok(Self {
            file: relative.to_string_lossy().replace('\\', "/"),
            realm: Self::text(relative.iter().next()),
            language,
            type_name,
            masked,
            methods,
        })
    }

    fn methods(
        relative: &Path,
        source: &str,
        utility: bool,
        language: &str,
        masked: &str,
    ) -> Result<Vec<PublicMethod>, Failure> {
        FsharpView::of(relative, source).map_or_else(
            || Ok(PublicMethods::find(language, masked)),
            |view| Self::fsharp_methods(&view, utility),
        )
    }

    fn fsharp_methods(view: &FsharpView, utility: bool) -> Result<Vec<PublicMethod>, Failure> {
        let masked_view = Self::masked(view.path(), view.text(), utility)?;
        Ok(FsharpMembers::find(view, &masked_view))
    }

    fn masked(relative: &Path, source: &str, utility: bool) -> Result<String, Failure> {
        let masked = crate::annotation_layout::masked_source(relative, source)?;
        if utility {
            return Ok(masked.replace(Self::STATIC_WORD, Self::STATIC_BLANK));
        }
        Ok(masked)
    }

    fn text(part: Option<&std::ffi::OsStr>) -> String {
        part.map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}
