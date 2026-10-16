use super::{FsharpMembers, PublicMethod, PublicMethods, RoleFiles};
use crate::Failure;
use crate::annotation_layout::UtilityLayout;
use crate::fsharp_source::FsharpView;
use crate::tac_cpp::CppCoverage;
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
        let read = Self::reader(relative);
        read(relative, source)
    }

    pub(crate) fn key(&self, method: &str) -> String {
        format!("{}.{method}", self.type_name)
    }

    fn reader(relative: &Path) -> fn(&Path, &str) -> Result<Option<Self>, Failure> {
        if crate::tac_cpp::CppSource::is_cpp(relative) {
            return Self::cpp;
        }
        Self::annotated
    }

    fn annotated(relative: &Path, source: &str) -> Result<Option<Self>, Failure> {
        let utility = UtilityLayout::is_utility(relative);
        let type_name = Self::type_name(relative, source, utility)?;
        type_name
            .filter(|_| RoleFiles::contains(relative))
            .map(|type_name| Self::with_type(relative, source, utility, type_name))
            .transpose()
    }

    fn cpp(relative: &Path, source: &str) -> Result<Option<Self>, Failure> {
        let coverage = RoleFiles::contains(relative)
            .then(|| CppCoverage::read(relative, source))
            .transpose()?
            .flatten();
        coverage
            .map(|coverage| Self::with_cpp(relative, source, coverage))
            .transpose()
    }

    fn with_cpp(relative: &Path, source: &str, coverage: CppCoverage) -> Result<Self, Failure> {
        let methods = coverage
            .methods
            .into_iter()
            .map(|(name, line)| PublicMethod::new(name, line))
            .collect();
        Ok(Self {
            file: relative.to_string_lossy().replace('\\', "/"),
            realm: Self::text(relative.iter().next()),
            language: String::from(crate::tac_cpp::CppSource::EXTENSION),
            type_name: coverage.class,
            masked: crate::annotation_layout::masked_source(relative, source)?,
            methods,
        })
    }

    fn type_name(relative: &Path, source: &str, utility: bool) -> Result<Option<String>, Failure> {
        let mut types = crate::annotation_layout::behaviour_types(relative, source)?;
        types.extend(Self::utility_name(relative, source, utility));
        Ok(types.into_iter().next())
    }

    fn utility_name(relative: &Path, source: &str, utility: bool) -> Option<String> {
        utility
            .then(|| crate::annotation_layout::ClassFileName::declared(relative, source))
            .flatten()
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
            || {
                Ok(PublicMethods::find(
                    crate::groovy_source::GroovySource::syntax(language),
                    source,
                    masked,
                ))
            },
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
